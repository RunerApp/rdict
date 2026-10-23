# 官方 rdict 数据改进方案：释义 ID、词组与数据质量

日期：2026-09-26
执行者：rdict 词典生成链路负责人（`/Users/azurefang/Source/rdict`）
使用方：Runer（查词浮窗、制卡、单词本关联）

## 0. 背景

Runer 会把 rdict 作为制卡的基础数据。用户从词典收藏一条释义后，单词本里的这条释义需要能长期找回它在词典中的原位置，用来：

- 在词典面板和浮窗里标记“✓ 已在单词本”，并把已收藏的释义置顶。
- 词典升级后提示“这条释义有更新”，由用户决定是否同步。
- 重装、换机、重新导入同一本词典后仍能匹配。

目前这件事做不到，因为 `Definition.id` 全部为空。本方案还列出了实测时发现的数据问题。

**不需要改 rdict 格式（spec v0.1）。** 所有改动都在生成链路：`headwords/db_export_*.py` → YAML → `compiler`。

## 1. 现状（2026-09-26 实测）

测量对象是 `/Users/azurefang/Source/rdict/` 下的 10 本官方词典，版本都是 0.1.0。

| 词典 | 词头数 | Sense 数 | Def 带 id | 例句数 | 例句无译文 | 例句无 targets | 同词同词性同译文 |
|---|---|---|---|---|---|---|---|
| en_zh | 122,799 | 142,547 | 0 | 141,531 | 5,598 | 2,501 | 384 |
| en_es | 122,799 | 142,547 | 0 | 141,531 | 5,450 | 2,501 | 1,706 |
| en_en | 122,799 | 142,547 | 0 | 141,531 | 不适用 | 2,501 | 不适用 |
| es_zh | 4,948 | 7,035 | 0 | 7,034 | 0 | 1,958 | 8 |
| es_en | 4,948 | 7,035 | 0 | 7,034 | 41 | 1,958 | 30 |
| ja_zh | 7,836 | 10,608 | 0 | 10,608 | 0 | 2,080 | 11 |
| ja_en | 7,836 | 10,608 | 0 | 10,608 | 0 | 2,080 | 12 |
| ja_es | 7,836 | 10,608 | 0 | 10,608 | 0 | 2,080 | 19 |

数据结构的现状：

- **一个 Sense 只有一个 Definition**，每个 Sense 平均 0.99–1.00 个。双语词典的 `Definition.value` 为空字符串，释义放在 `Sense.translations`。单语词典的释义放在 `Definition.value`。
- 双语词典的 `Sense.translations` 就是短母语释义，ja/es→zh 平均 4–5 个字。Runer 会直接用它做 ruby 注释和卡片正面，**不需要另加 gloss 字段**。
- 词组：rdict 中没有 `Relation{type:"phrase"}`，多词词头在 en 词典里只有 1 个。但源库 `dictionary.db` 的 `phrases` 表里已经有 **92,938 条**词组（例如 `run out of`、`run out of steam`），它们没有对应的 entry，也没有释义。

## 2. 任务 A：释义稳定 ID

### 2.1 格式

**id 在词条内唯一，不需要全局唯一。** Runer 用 `(dictKey, headword, defId)` 组成全局键，其中 `dictKey = manifest.name + source_lang`。所以 id 本身越短越好。

```
<pos 缩写><在该词性内的序号>
```

- pos 缩写取固定映射：NOUN→n、VERB→v、ADJ→a、ADV→r、PRON→pr、AUX→aux、ADP→p、DET→d、NUM→num、INTJ→i、CCONJ→cc、SCONJ→sc、PART→pt、PROPN→pn，其他一律→x。
- 例如 `run` 的 6 个 Sense：`v1 v2 v3 n1 v4 n2`（按当前出现顺序，各词性独立计数）。
- 多词源的词条（etymologies 大于 1）加前缀 `e2.`，例如 `e2.n1`。第一个词源不加前缀。
- 以后如果一个 Sense 有多个 Definition，用 `v1.2` 表示第 2 条。

### 2.2 稳定性规则（最重要）

**id 一旦发布就不能再指向别的意思。** 用序号生成只能用在第一次生成，之后必须把 id 存回源库：

1. 给 `dictionary.db` 的 `definitions` 表加一列 `public_id TEXT`，并建唯一约束 `(entry_id, public_id)`。
2. 第一次运行时按 2.1 的规则回填，之后导出时**只读这一列，不再重新计算**。
3. 新增的 Sense 取该词性当前的最大序号加 1。**删除后序号不复用。**
4. 调整 Sense 顺序只改 `sort_order`，不改 `public_id`。
5. 合并、改名或删除已发布的义项时，写一条 id 重定向记录，格式见 §8.2。Runer 据此迁移单词本里的关联。

同一 headword 在不同语言对的词典里（en_zh 与 en_es）**必须使用同一套 id**。它们来自同一张 `definitions` 表，所以天然满足。用户换母语词典时，关联也不会断。

### 2.3 体积

在 en_zh（9.7MB，14 万条释义）上模拟：

| 方案 | 原始大小 | zstd-19 后 | 占文件比例 |
|---|---|---|---|
| 词条内局部 id（`v1`、`n2`） | 428 KB | **47 KB** | 0.5% |
| 带 headword 的全局 id（`run#v1`） | 1.78 MB | 404 KB | 4% |
| 随机 id（8 字符） | 1.28 MB | 944 KB | 9% |

结论：**采用局部 id，体积可以忽略。**

### 2.4 manifest

最终格式见 §8.1。

### 2.5 验收

- 10 本词典里每个 Definition 都有 id，并且在词条内唯一。
- 从同一个源库导出两次，id 完全一致。
- 对某个词条调整 Sense 顺序后重新导出，所有 id 不变。
- en_zh、en_es、en_en 中同一词条同一 Sense 的 id 相同。

### 2.6 验收结果（2026-09-26 复测）

- 12 本词典（含两本 test）共 141,513 / 7,035 / 10,608 条释义，**全部带 id**，格式都符合 `pos-seq`。
- 各语言对之间 id 完全一致：en_zh、en_es、en_en、en_enzh_test 的 141,513 条全部相同；es、ja 各组也全部一致。
- 源库 `definitions.public_id` 已存在。
- 带括号的污染词头已清理，词头数从 122,799 降到 122,713。
- **还有问题：53 个 en 词条内部有重复 id。** 大部分是变体拼写合并进同一个词头后，两边各自带着 `a1` / `p1`。例如：
  - `among` 有两个 p1：“在…之中”和“among（amongst的变体拼写）”
  - `alphabetic` 有两个 a1，译文完全相同
  - `agronomic`、`amid`、`analytic`、`armored` 同类

  处理方式：合并后给后来的那条重新分配序号，比如 `a2`。如果是纯重复，就直接删掉（这一轮不需要写重定向记录，见 §7.1）。
- manifest 还没有 `official`、`id_scheme`、`data_version` 字段，Runer 目前只能凭 id 格式推断。

## 3. 任务 B：词组

Runer 的浮窗会根据句子的 lemma 序列匹配词组，命中后把词组显示在最上面（例如 “had run out of patience” 中的 `run out of`）。这需要两样东西：

1. **词组是词头**：`run out of` 自己是一个 entry，有 Sense、translations 和例句。
2. **成分词反向链接**：`run` 的 entry 带 `Relation{type:"phrase", target:"run out of"}`。spec §8.3 已经定义了这种方式。

### 3.1 数据来源

`dictionary.db.phrases` 里有 92,938 条，但质量参差不齐：

- 包含 **5,937 条**带 `sy / sg / sb / sth` 占位符的条目，例如 `absolve sy of sg`。这是匈牙利语词典的缩写习惯（sy=somebody、sg=something）。
- 有明显的噪声或拼写错误，例如 `run out a warpe`、`2 binary 1 quarternary code`。

### 3.2 建议

- **第一批不追求全量。** 先按词频挑出约 5,000–8,000 条常用短语动词和固定搭配（run out of、give up、look forward to…），生成 Sense、translations 和例句后再进 rdict。
- **占位符统一**成 `sb / sth`，并在 Runer 端当作通配符匹配：`absolve sb of sth` 可以匹配 “absolved him of blame”。匹配规则由 Runer 实现，数据侧只需要保证写法一致。
- 词组的 Sense 同样按任务 A 分配 id。
- 分离式短语（如 “turn it off”）由 Runer 匹配时允许中间插入最多 2 个词，数据侧不需要特殊处理。

## 4. 任务 C：数据质量问题清单

按对 Runer 的影响排序。

### 4.1 词头污染（高）

en 词典中有 **156 个带括号的词头**，大多是缩写标记被误拆成了词头，例如：

- `(anat.)`、`(bot.)`、`(colloq.)`
- 更糟的是 `(colloq.)uial`、`(colloq.)uium`、`(colloq.)uy`。看起来是某次替换把 `colloq` 当作缩写展开，把 “colloquial” 这类词弄坏了。例句是正常的，例如 “His speech was full of colloquial expressions.”，但词头已经坏了。
- `(i.)q.`

处理方式：删掉纯标记词头；把被破坏的词头恢复成原词（colloquial、colloquium、colloquy），并与已有同名词条合并。

另外，`PUNCT` 6 条、`SYM` 6 条、`X` 414 条中，大部分是 `a-`、`ab-` 这类词缀，或 `...`。词缀可以保留，但应该给 `tags` 加上 `type:affix`，Runer 就不会在查词浮窗里匹配它们。

### 4.2 近重复义项（中）

同一词条、同一词性、相同译文的 Sense：en_zh 384 组，en_es 1,706 组。例如：

- `ability` NOUN “才能；本领” 出现两次
- `abyss`、`advantage`、`amendment` 同上
- `harbor` 的 v1≈v3（港湾/港口）、v2≈v4（庇护，怀有/怀有，持有）。这组译文并不完全相同，但意思重复，需要语义去重。

处理方式：先合并译文完全相同的，再用 embedding 找出语义重复的交给人工审核。合并时遵守 2.2 的规则 5。

### 4.3 例句缺译文（中）

en_zh 有 5,598 条、en_es 有 5,450 条例句没有 `translations`。Runer 在制卡时会把例句译文作为兜底翻译，缺失时要额外调用 AI。

### 4.4 例句缺 targets（中）

各词典都有例句的 `targets` 为空：en 2,501 条，es 1,958 条，ja 2,080 条。Runer 用 targets 高亮例句中的目标词，并用于挖空卡。缺失时只能靠字符串匹配，对屈折变化（ran / running）和日语活用形都不可靠。

日语尤其明显：`食べる` 的第一个例句 “毎朝、パンを食べます。” 就没有 targets。需要用形态分析器（sudachi 或 kuromoji）回填。

另外请**在 spec 中写明 `TextSpan.offset` 的单位**（UTF-8 字节还是 UTF-16 码元）。从实测看，en 的 offset 与字符位置一致，但 ja 的 `この仕事は時間を食べる。` 给出的是 offset 24、length 9，是 UTF-8 字节。不写清楚的话，各端实现容易出错。

### 4.5 日语读音字段（中）

ja 词典的发音全部写成 `kind:"ipa"`，值为 `"たべる (taberu)"`，把假名和罗马音混在了一个字段里，而且都不是 IPA。

改为两条：

```yaml
pron:
  - { lang: ja, kind: kana, value: たべる }
  - { lang: ja, kind: romaji, value: taberu }
```

Runer 会用 kana 显示假名注音，用 romaji 做可选的辅助显示。有条件的话，可以再加一条 `kind: pitch` 表示声调核。

**已落地（2026.9.29 重构）**：DB 新增泛化表 `entry_prons(entry_id, kind, value, sort_order)`，所有语言的读音入库时就按 kind 拆好（ja → `hira`+`roma`、ko → `roma`（RR 罗马字）、en/es/fr → `ipa`）。编译器**不再做任何语言特殊处理**（`split_ja_reading` 已删除），只查表透传。`entries.ipa` 列降级为原始来源参考，入库脚本（如 `db_hangul_roman.py`）改写 `entry_prons`。迁移脚本：`headwords/db_migrate_prons.py`。

### 4.6 日语词头变体写法（低）

ja 词典有 319 个词头带 `〜`、`～`、括号、分号或空格，例如 `(かさを～) さす`、`いい; よい`、`〜(日本) 式`。

- 分号分隔的写法（`いい; よい`）应该拆成两个词头，或者主词头加 `Form` / `see` 引用。
- 括号里的搭配提示应该移到 Sense 的 `tags` 或 `Note` 里，词头只保留 `さす`。
- 否则用户点 “さす” 时查不到。

### 4.7 其他（低）

- `Relation.type_` 出现了 1 条 `com_type`（`co-` → `co`），不在 spec §6.11 的枚举里，需要确认是否是误用。
- 双语词典的 `Definition.value` 都是空字符串。建议改成省略，或者直接写成英英释义的原文，这样双语词典也能显示英文解释，不用同时装 en_en。**更推荐第二种**，这样一本 en_zh 就能提供“中文粗体 + 英文补充”两行。实测 `dictionary_en_enzh_test.rdict` 已经是这种结构，建议作为正式版本。
- 频次标签只覆盖了 2,809 个词头（NGSL 范围）。浮窗和“新词提示”需要更广的频次覆盖，可以考虑补 SUBTLEX 或 wordfreq 的排名，以 `freq:rank=N` 的形式写入 tags。

## 5. 建议顺序

1. 任务 A：id 回填，加上 manifest 字段。这是 Runer 制卡关联的前置条件。
2. 4.1 词头污染，以及 4.7 里双语词典写入英英释义。
3. 4.5 日语读音，4.4 targets 回填。
4. 4.2 近重复义项合并。
5. 任务 B：词组第一批。
6. 其余问题。

## 6. Runer 侧对应改动（供对照，不在本任务范围）

- `word_senses` 和 `phrase_senses` 增加 `dict_ref JSON`：`{namespace, headword, defId, dataVersion, textHash}`。`namespace` 取 §8.1 的 `id_namespace`，不再用词典名称。
- 读到 `runer.id_scheme` 时按 id 匹配；没有时按 `(etyIndex, senseIndex)` 加 `textHash` 做模糊匹配。
- 按 §8.2 处理 `id_redirects`，迁移用户的旧关联；按 §8.3 检查更新。

## 7. 待修复问题（2026-09-26 复测后追加）

### 7.1 en 词条内部 id 重复

**影响**：en_zh、en_es、en_en 以及两本 test 词典。这几本共用同一套 id，所以问题完全相同。

**现象**：共 51 组，其中 11 组的译文完全相同，40 组不同。同一词条内有两条释义带同一个 id。用户收藏其中一条后，Runer 无法区分是哪一条，“✓ 已在单词本”会同时标在两条上，词典更新检测也会错乱。

**原因推测**：变体拼写（amongst→among、agronomical→agronomic 等）合并进同一个词头时，两边各自带着原来的 `public_id`，而且都是该词性下的第一条（`a1`、`p1`）。

**处理规则**：

1. 译文完全相同（下表“完全相同”，11 组）：删掉后出现的那条。

   **这一轮修复不需要写重定向记录。** Runer 还没有发布按 id 关联的功能，用户数据里不存在这些 id。另外，当前词典没有 `runer.id_scheme`，Runer 不会信任其中的 id（见 §8.1）。所以修复后的版本就是 id 的**基线**。从基线开始，任何合并、改名、删除都必须按 §8.2 写记录。
2. 译文不同（40 组）：保留第一条的 id，后面的重新分配为该词性当前最大序号加 1，例如 `a1` → `a2`，并写回 `definitions.public_id`。
3. “X 的变体拼写”这类释义（如 `among（amongst的变体拼写）`），建议不作为独立义项，改成 `Form{kind:"variant", term:"amongst"}` 挂在词条上，然后按规则 1 删除。基线之后再有这种合并，就按 §8.2 写 `{from:["amongst","p1"], to:["among","p1"], reason:"merged"}`。
4. 修复后，在源库上给 `(entry_id, public_id)` 加唯一约束。这样以后合并词条时，重复 id 会直接报错。

**验收**：所有词典都满足“每个词条内 `Definition.id` 唯一”，并且与本次发布相比，**没有出问题的词条的 id 一个都不变**。

**完整清单**：

| 词头 | 重复 id | 条数 | 译文 | 各条译文 |
|---|---|---|---|---|
| agronomic | a1 | 2 | 不同 | 农艺学的 / agronomical的另一种拼写形式；农艺学的 |
| alphabetic | a1 | 2 | 完全相同 | 字母的；按字母顺序的 / 字母的；按字母顺序的 |
| amid | p1 | 2 | 不同 | 在……期间；在……过程中；通过 / 在中间；被……包围的（ amid 的变体拼写） |
| among | p1 | 2 | 不同 | 在…之中；在…中间 / among（amongst的变体拼写） |
| analytic | a1 | 2 | 不同 | 分析的；解析的 / 分析的；分析性的；逻辑推理的 |
| armored | a1 | 2 | 不同 | 装甲的；有防护层的 / 装甲的；披甲的 |
| armorer | n1 | 2 | 不同 | 盔甲匠；军械师 / 盔甲匠；军械匠 |
| armoring | v1 | 2 | 不同 | 装甲；加防护层 / 为…装甲；装备防护层 |
| armory | n1 | 2 | 不同 | 军械库；兵工厂 / 军械库；盔甲库 |
| bosun | n1 | 2 | 不同 | 水手长 / 水手长（bosun 的旧拼法变体） |
| briquet | n1 | 2 | 不同 | 煤饼；块状燃料 / 煤砖；压块燃料 |
| bull's-eye | n1 | 2 | 不同 | 靶心；正中目标 / 靶心（bull's-eye 的旧式排印） |
| caviar | n1 | 2 | 不同 | 鱼子酱 / 鱼子酱（caviar 的旧拼法之一） |
| colloquial | a1 | 2 | 不同 | 口语的 / 涉及或具有非正式交谈特征的。 |
| colloquium | n1 | 2 | 不同 | 学术研讨会 / 交谈；对话（罕见或古旧形式）。 |
| cyclopedia | n1 | 2 | 完全相同 | 百科全书（变体） / 百科全书（变体） |
| downward | r1 | 2 | 不同 | 向下地 / 向下地（含括号变体形式） |
| ear | n1 | 3 | 不同 | 耳朵 / 耳朵；听力 / 穗（禾谷作物） |
| earnest | a1 | 2 | 完全相同 | 认真的；诚挚的 / 认真的；诚挚的 |
| electric | a1 | 2 | 不同 | 电的 / 电的；电动的 |
| encyclopedia | n1 | 2 | 不同 | 百科全书（美式拼写） / 百科全书（两种拼写） |
| enrol | v1 | 2 | 不同 | 注册；登记（英式拼写） / 注册（两种拼写联排） |
| epaulet | n1 | 2 | 不同 | 肩章 / 肩章（拼写变体） |
| ethnic | a1 | 2 | 不同 | 种族的；民族的；异域风情的 / 民族的（拼写并列条目） |
| fagot | n1 | 2 | 不同 | 柴捆（变体拼写） / 柴捆；束柴 |
| fledgling | n1 | 2 | 完全相同 | 初飞幼鸟；新手 / 初飞幼鸟；新手 |
| gyps | n1 | 2 | 不同 | 石膏；石膏粉（复数/缩写） / 石膏 |
| instalment | n1 | 2 | 不同 | 分期付款；一期 / 分期付款；一期（部分） |
| instil | v1 | 2 | 不同 | 灌输；逐渐灌输 / 灌输（变体） |
| ironic | a1 | 2 | 不同 | 讽刺的；具有讽刺意味的 / 讽刺的（变体） |
| its | pr1 | 2 | 不同 | 它的 / 它的；它是 |
| jasmin | n1 | 2 | 不同 | 茉莉（变体） / 茉莉花 |
| jiff | n1 | 2 | 不同 | 瞬间；一会儿 / 瞬间（变体） |
| judea | pn1 | 2 | 完全相同 | 犹太地（古地区） / 犹太地（古地区） |
| judgment | n1 | 2 | 不同 | 判断 / 判断；裁决；判决 |
| juristic | a1 | 2 | 不同 | 法学的；法律上的 / 法学的（变体） |
| kilogram | n1 | 2 | 不同 | 千克；公斤 / 千克（两种拼写） |
| milage | n1 | 2 | 完全相同 | 里程；英里数 / 里程；英里数 |
| nor | cc1 | 2 | 不同 | 也不；既不 / nor 的缩写或缩略形式。 |
| p. | n1 | 3 | 不同 | 页 / 第(i)页 / 第(s)页 |
| succor | n1 | 2 | 不同 | 援助；救济 / 援助 |
| succor | v1 | 2 | 不同 | 援助；救济 / 援助 |
| synoptic | a1 | 2 | 不同 | 概要的；对观福音的 / 概要的 |
| theoretic | a1 | 2 | 完全相同 | 理论的 / 理论的 |
| thill | n1 | 2 | 不同 | 车辕 / 辕马 |
| traveling | v1 | 2 | 不同 | 中 /  |
| upward | a1 | 2 | 完全相同 | 向上的 / 向上的 |
| upward | r1 | 2 | 完全相同 | 向上 / 向上 |
| w. | r1 | 2 | 不同 | 编辑标记中 'so write' 或 'see above' 的缩写。 / 手稿校勘中 'same as above' 或 'similiter' 的缩写。 |
| whodunit | n1 | 2 | 完全相同 | 侦探小说 / 侦探小说 |
| wizen | v1 | 2 | 完全相同 | 干瘪 / 干瘪 |

### 7.2 manifest 缺少官方元数据

**影响**：全部 12 本词典。

**现象**：`manifest.json` 中没有 `official`、`id_scheme`、`data_version`、`update_url`。Runer 只能根据 id 的格式猜测这本词典是否有稳定 id，用户自己导入的第三方 rdict 也可能碰巧用了同样的格式。

**要求**：按 §8.1 写入 `runer` 对象，按 §8.3 生成更新索引。

**验收**：见 §8.5。

## 8. 官方元数据格式（定稿）

本节取代前文中所有关于 manifest 新字段、`merged` 和 `update_url` 的草案。

### 8.1 manifest 中的 `runer` 对象

所有 Runer 专用字段都放进 `manifest.json` 顶层的**一个 `runer` 对象**里，不散落在顶层。spec §3.3 规定读取器必须忽略未知的顶层 key，所以旧版读取器不受影响。

```json
{
  "format": "rdict",
  "pack": { "name": "English–Chinese Dictionary (英汉词典)", "version": "0.1.0", "source_lang": "en", "target_langs": ["zh-Hans"] },
  "data": { "...": "..." },

  "runer": {
    "schema": 1,
    "official": true,
    "pack_id": "runer.en-zh",
    "id_namespace": "runer:en",
    "id_scheme": "pos-seq-v1",
    "data_version": "2026.11.1",
    "update_url": "https://runerapp.com/api/dicts/index",
    "id_redirects": []
  }
}
```

| 字段 | 类型 | 必填 | 规则 |
|---|---|---|---|
| `schema` | int | 是 | 这个对象本身的版本，目前为 `1`。Runer 遇到更大的值时只读取认识的字段。 |
| `official` | bool | 是 | 官方词典写 `true`。**这个字段只是声明。**Runer 只有在文件哈希与签名索引（§8.3）一致时，才显示“官方”标识。 |
| `pack_id` | string | 是 | 文件的身份，格式为 `runer.<源语言>-<目标语言>`，例如 `runer.en-zh`、`runer.en-en`、`runer.ja-zh`。双释义词典写 `runer.en-en+zh`。发布后不能改。 |
| `id_namespace` | string | 是 | **id 的身份**，格式为 `runer:<源语言>`。共用同一张 `definitions` 表的词典写同一个值：en_zh、en_es、en_en 都是 `runer:en`。Runer 用 `(id_namespace, headword, defId)` 关联，所以用户换母语词典后关联不会断。 |
| `id_scheme` | string | 是 | `pos-seq-v1`，表示 id 遵守 §2.1 和 §2.2。**没有这个字段时，Runer 不信任 `Definition.id`。** |
| `data_version` | string | 是 | 格式 `YYYY.MM.N`，按三段数字比较，只增不减。**同一 `id_namespace` 下的所有 pack 必须同时发布，并使用同一个 data_version。** |
| `update_url` | string | 是 | 固定为 `https://runerapp.com/api/dicts/index`，所有官方 pack 都写同一个地址，见 §8.3。 |
| `id_redirects` | array | 是 | 可以是空数组。格式见 §8.2。 |

`pack.version` 仍然表示包格式版本，和 `data_version` 无关。

**需要 rdict 库配合**：`@runer-hq/rdict` 的 `manifest()` 返回的是固定类型的 `Manifest`，读不到 `runer` 字段。请在 Node 绑定里加一个 `manifestJson(): string`，返回 `manifest.json` 的原文，Runer 自己解析。在它发布之前，Runer 先用 `adm-zip` 直接读取 `manifest.json` 这个 ZIP 成员：它是 store 方式存储的，读取成本很低。

### 8.2 `id_redirects`：id 重定向记录

id 发布后，如果某条义项被合并、改名或删除，就在这里写一条记录。它取代前文的 `merged`。

```json
"id_redirects": [
  { "from": ["amongst", "p1"], "to": ["among", "p1"], "reason": "merged", "since": "2026.12.1" },
  { "from": ["harbor", "n2"], "to": ["harbor", "n1"], "reason": "merged", "since": "2026.12.1" },
  { "from": ["colour", "v3"], "to": null, "reason": "deleted", "since": "2027.01.1" }
]
```

| 字段 | 规则 |
|---|---|
| `from` | `[headword, defId]`：旧位置。 |
| `to` | `[headword, defId]`：新位置。允许跨词头，覆盖变体拼写合并和词头更正。删除且没有替代项时写 `null`。 |
| `reason` | `merged`（合并进另一条）、`moved`（词头改名或换了词条）、`split`（一条拆成多条，`to` 指向主要的那条）、`deleted`。 |
| `since` | 第一次出现这条重定向的 `data_version`。 |

规则：

1. **累积保存，永远不删。** 用户可能跳过好几个版本才升级，所以每个版本都带上从基线开始的全部记录。
2. **链式记录由 Runer 解析。** 比如 A→B、B→C，Runer 顺着链找到最终位置，最多跳 8 次，遇到循环就停下，按 `textHash` 模糊匹配。数据侧也可以直接把 A 改写成 A→C，两种都可以。
3. **`from` 不能是当前版本里仍然存在的位置。** 导出脚本需要校验这一点：如果 `from` 指向的词条和 id 还在，说明这条重定向有误。
4. 在源库里建一张表：`id_redirects(from_headword, from_id, to_headword, to_id, reason, since, PRIMARY KEY(from_headword, from_id))`。合并或删除时写入这张表，导出时整表输出。
5. 体积：每条约 80 字节，1 万条约 800KB 的 JSON。manifest 本身不压缩，所以超过 **5,000 条**时要拆出去，由 `runer.id_redirects_member: "runer/id-redirects.json"` 指向一个单独的 ZIP 成员。第一年预计远远达不到。

**范围限定**：`id_redirects` 只在同一个 `id_namespace` 内有效。

### 8.3 更新索引（`update_url`）

所有官方 pack 共用一个索引，一次请求就能拿到全部版本。这个索引同时用于**新手引导里按语言对安装官方词典**，以及词典设置页的“检查更新”。

**地址**：`GET https://runerapp.com/api/dicts/index`，托管在 web 项目里，与 `app/api/app-policy` 并列。

**响应**：签名结构沿用 app-policy，用 Ed25519 对 payload 原始字节签名。**使用一对单独的密钥**（`DICT_INDEX_PRIVATE_KEY`），客户端内置对应的公钥。这样即使 app-policy 的私钥泄露，也不能伪造词典索引，验签代码可以复用。

```json
{ "payload": "<base64 JSON>", "signature": "<base64 Ed25519>" }
```

解码后的 payload：

```json
{
  "schema": 1,
  "generated_at": "2026-11-02T08:00:00Z",
  "packs": [
    {
      "pack_id": "runer.en-zh",
      "id_namespace": "runer:en",
      "data_version": "2026.11.1",
      "source_lang": "en",
      "target_langs": ["zh-Hans"],
      "name": { "zh": "英汉词典", "en": "English–Chinese", "es": "Inglés–Chino" },
      "entry_count": 122713,
      "format_version": "0.1",
      "min_app_version": "0.3.0",
      "url": "https://dicts.runerapp.com/runer.en-zh/2026.11.1.rdict",
      "size": 10171311,
      "sha256": "9f2c…",
      "released_at": "2026-11-01T00:00:00Z",
      "changelog": { "zh": "修复 51 处重复义项 id；补充 5,600 条例句译文。", "en": "…" }
    }
  ]
}
```

| 字段 | 规则 |
|---|---|
| `url` | **不可变的版本化地址**：一个 URL 对应一个版本，永不覆盖，可以长期缓存。托管位置不限（R2、S3 或 GitHub Releases 均可），只要支持 HTTPS 和 Range 请求（断点续传）。 |
| `sha256` | 整个 `.rdict` 文件的哈希。下载后必须校验。Runer 也用它判断本地文件是否为官方原版（§8.1 `official`）。 |
| `min_app_version` | 低于这个版本的 Runer 不提供更新，只在列表里显示“需要升级 Runer”。 |
| `format_version` | rdict 格式版本。Runer 不支持的格式直接跳过。 |
| `name`、`changelog` | 按界面语言取值，缺失时退回 `en`。 |

缓存：索引 `s-maxage=300`，文件由 CDN 长期缓存。

**Runer 客户端行为**（供对照）：

1. 每 24 小时最多检查一次；用户在设置里点“检查更新”时立即检查。
2. 本地 pack 的 `data_version` 小于索引中的版本时，提示“有新版本”并附上 changelog，**不自动下载**，因为词典有几 MB 到几十 MB。
3. 下载到临时目录，依次校验 sha256 和 manifest 中的 `pack_id` / `data_version` 与索引一致，然后原子替换文件，并更新 `dictionaries.file_path`。失败时保留旧文件。
4. **同一 `id_namespace` 的 pack 一起提示更新**，避免 en_zh 和 en_en 处在不同版本。
5. 替换后按 `id_redirects` 迁移 `dict_ref`，并对 `textHash` 变化的收藏标记“词典有更新”。

### 8.4 发布流程

1. 源库修改完成后，为同一 `id_namespace` 的所有 pack 统一设置新的 `data_version`。
2. 导出并校验（§8.5），上传到 `url` 对应的版本化地址。
3. 在 web 项目的 `app/api/dicts/packs.production.json` 里更新这些 pack 的条目。发布方式和 app-policy 一样，走 Vercel 部署。
4. 部署后用 `curl` 验证签名和 sha256。

### 8.5 验收

- 12 本词典的 manifest 都有完整的 `runer` 对象，§8.1 的必填字段齐全。
- 同一 `id_namespace` 下的 pack，`data_version` 相同。
- 每个词条内 `Definition.id` 唯一（即 §7.1 修复完成）。
- 导出脚本对 `id_redirects` 做过规则 3 的校验，基线版本可以是空数组。
- 索引 payload 能用公钥验签，每个 pack 的 `sha256` 与下载到的文件一致。

## 9. 工作流约定（2026-09-28 定稿）

### 9.1 生成流程：先母语，后翻译

新词典生成分两步走，不再一步到位：

1. **第一步（生成时）**：只生成**源语言（母语）释义和例句**，不做任何目标语言翻译。
2. **第二步（校对后）**：母语内容**充分校对**之后，再批量生成各目标语言的翻译。

理由：翻译错误大多源于源释义的歧义或错误，先校对源头，翻译的质量和一致性才有保障；也避免源内容改动后翻译全部作废重跑。

### 9.2 翻译覆盖语言清单（目标语言全集）

翻译要覆盖以下 11 种语言（按需扩展，此为基线清单）：

| # | 语言 | BCP-47 | 备注 |
|---|---|---|---|
| 1 | 汉语（简体） | zh-Hans | |
| 2 | 汉语（繁体） | zh-Hant | 由 zh-Hans 经 OpenCC 规则转换，不走模型 |
| 3 | 英语 | en | |
| 4 | 德语 | de | |
| 5 | 法语 | fr | |
| 6 | 日语 | ja | |
| 7 | 西班牙语 | es-ES | |
| 8 | 意大利语 | it | |
| 9 | 葡萄牙语 | pt | 2026.9.30 加入 |
| 10 | 越南语 | vi | 2026.9.30 加入 |
| 11 | 印尼语 | id | 2026.9.30 加入 |

现状对照：已覆盖 zh-Hans / en / es-ES / ja（含 ja→es、en→es 等）；**de / it / pt / vi / id 尚未有任何翻译数据**，后续校对完成后补齐。zh-Hant 不入库（由发布管线从 zh-Hans 规则转换），或按需要单独存储。

### 9.3 词频标签命名

- 法语 Cobb 2026 词表的频次标签命名修正为 **`freq:lfnf`**（LFNF，词表正式名称），此前误写为 `freq:cobb`。
  - 分级：`freq:lfnf=list_1..list_3`
  - 频次：`freq:lfnf.freq=N`
- 已入库的 `freq:cobb*` 标签由脚本统一改名后重编包。
- 韩语《국립국어원 한국어 학습용 어휘 목록》（2003，5,965 行 A/B/C 等级）用 **`freq:klvl`**：
  - 分级：`freq:klvl=list_A..list_C`
  - 频次：`freq:klvl.freq=N`（词表 순위 列）

### 9.4 释义发布状态（definitions.published）

- `definitions.published`：`0` = 未发布（public_id 可随意调整、义项可增删改），`1` = 已发布（public_id 冻结，遵守 §2.2 稳定性规则，改动必须走 `id_redirects`）。
- **当前所有语言的释义均未发布（published=0）**，发布前可全局重排 id。
- 发布某个 pack 时，把对应语言的释义标记为 published=1；此后重编号脚本只动未发布的。
- 维护脚本：`headwords/db_renumber_public_ids.py`（按 sense.sort_order 重排未发布释义的 public_id）。
