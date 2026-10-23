import Foundation
import SwiftUI

// MARK: - Localization

enum AppLang: String, CaseIterable {
    case en = "English"
    case zh = "中文"
}

struct L10n {
    static var lang: AppLang = .en

    static func setLang(_ l: AppLang) {
        lang = l
        UserDefaults.standard.set(l.rawValue, forKey: "rdict.lang")
    }

    static func restore() {
        if let saved = UserDefaults.standard.string(forKey: "rdict.lang"),
           let l = AppLang(rawValue: saved) {
            lang = l
        }
    }

    // ── Strings ──────────────────────────────────────────────────────
    static var appName: String { lang == .en ? "Rdict" : "Rdict" }
    static var noDict: String { lang == .en ? "No Dictionary Loaded" : "未加载词典" }
    static var noDictHint: String { lang == .en ? "Open a .rdict file to start browsing" : "打开 .rdict 文件开始查词" }
    static var openDict: String { lang == .en ? "Open Dictionary…" : "打开词典…" }
    static var search: String { lang == .en ? "Search…" : "搜索…" }
    static var selectWord: String { lang == .en ? "Select a word" : "选择一个词" }
    static var dictionaries: String { lang == .en ? "Dictionaries" : "词典" }
    static var entries: String { lang == .en ? "entries" : "词条" }
    static var close: String { lang == .en ? "Close" : "关闭" }
    static var error: String { lang == .en ? "Error" : "错误" }
    static var related: String { lang == .en ? "Related Words" : "相关词" }
    static var wordStructure: String { lang == .en ? "Word Structure" : "词形结构" }
    static var open: String { lang == .en ? "Open" : "打开" }
    static var language: String { lang == .en ? "Language" : "语言" }
    // Detail page
    static var morphology: String { lang == .en ? "Morphology" : "词形" }
    static var examples: String { lang == .en ? "Examples" : "例句" }
    static var definitions: String { lang == .en ? "Definitions" : "释义" }
    static var translations: String { lang == .en ? "Translations" : "翻译" }
    static var etymology: String { lang == .en ? "Etymology" : "词源" }
    static var forms: String { lang == .en ? "Forms" : "词形变化" }
    static var notes: String { lang == .en ? "Notes" : "注释" }
    // Morpheme kinds
    static var prefix: String { lang == .en ? "prefix" : "前缀" }
    static var suffix: String { lang == .en ? "suffix" : "后缀" }
    static var root: String { lang == .en ? "root" : "词根" }
    static var combiningForm: String { lang == .en ? "combining form" : "构词成分" }
    static var particle: String { lang == .en ? "particle" : "助词" }
    static var stem: String { lang == .en ? "stem" : "词干" }

    static func morphemeKind(_ kind: String?) -> String {
        guard let kind = kind?.lowercased() else { return "" }
        switch kind {
        case "prefix": return prefix
        case "suffix": return suffix
        case "root": return root
        case "combining_form", "combining form": return combiningForm
        case "particle": return particle
        case "stem": return stem
        default: return kind
        }
    }

    // Relation types
    static var synonym: String { lang == .en ? "SYN" : "同义" }
    static var antonym: String { lang == .en ? "ANT" : "反义" }
    static var derived: String { lang == .en ? "DER" : "派生" }

    static func relationType(_ type: String?) -> String {
        guard let type = type?.uppercased() else { return "" }
        switch type {
        case "SYN": return synonym
        case "ANT": return antonym
        case "DER": return derived
        default: return type
        }
    }
}
