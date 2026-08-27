export interface Morpheme {
  kind: string | null;
  term: string;
}

export interface MediaRef {
  kind: string;
  hash: string;
  description: string | null;
  alt: string | null;
}

export interface Pron {
  lang: string | null;
  accent: string | null;
  kind: string | null;
  value: string;
  media: MediaRef[];
}

export interface Translation {
  lang: string | null;
  value: string;
  pron: Pron[];
}

export interface Form {
  kind: string | null;
  term: string;
  tags: string[];
  feats: string | null;
  pron: Pron[];
}

export interface TextSpan {
  offset: number;
  length: number;
}

export interface TargetOccurrence {
  spans: TextSpan[];
}

export interface Example {
  value: string;
  translations: Translation[];
  pron: Pron[];
  targets: TargetOccurrence[];
  media: MediaRef[];
}

export interface Note {
  id: string | null;
  value: string;
  examples: Example[];
}

export interface Definition {
  id: string | null;
  value: string;
  examples: Example[];
  notes: Note[];
  media: MediaRef[];
}

export interface DefGroup {
  id: string | null;
  description: string | null;
  definitions: Def[];
}

export type Def =
  | { Definition: Definition }
  | { Group: DefGroup };

export interface Sense {
  pos: string | null;
  lemma: string | null;
  translations: Translation[];
  forms: Form[];
  tags: string[];
  pron: Pron[];
  definitions: Def[];
}

export interface Ety {
  id: string | null;
  root: string | null;
  senses: Sense[];
}

export interface Relation {
  type_: string | null;
  target: string;
}

export interface Entry {
  headword: string;
  see: string | null;
  tags: string[];
  pron: Pron[];
  etymologies: Ety[];
  morphology: Morpheme[];
  relations: Relation[];
  media: MediaRef[];
}

export interface Manifest {
  name: string;
  version: string;
  source_lang: string;
  target_langs: string[];
  entry_count: number;
  block_count: number;
  cover?: string;
}

export interface MediaManifestEntry {
  hash: string;
  kind: string;
  ext: string;
  mime: string;
  compression: string;
  size: number;
  uncompressed_size: number;
}

export interface MediaKey {
  kind: 'audio' | 'image' | 'video';
  hash: string;
}

export interface MediaInfo {
  hash: string;
  kind: string;
  ext: string;
  mime: string;
  compression: string;
  size: number;
  uncompressed_size: number;
}

export declare class Dictionary {
  constructor(path: string);
  lookup(headword: string): Entry | null;
  listHeadwords(): string[];
  prefix(prefix: string, limit?: number): string[];
  manifest(): Manifest;
  mediaManifest(): MediaManifestEntry[] | null;
  mediaInfo(key: MediaKey): MediaInfo | null;
  readMedia(key: MediaKey): Buffer;
  extractMedia(key: MediaKey, outputPath: string): number;
  readCover(): Buffer | null;
}
