export type Inline = {inline:'text'|'code';text:string}|{inline:'strong'|'emphasis';text:Inline[]}|{inline:'break'}|{inline:'link';text:Inline[];to:{target:string;page?:string;anchor?:string;url?:string}};
export type Block =
  | {block:'prose';text:Inline[]}
  | {block:'section';level:number;anchor:string;title:Inline[];blocks:Block[]}
  | {block:'table';columns:Inline[][];rows:Inline[][][]}
  | {block:'list';ordered:boolean;items:Block[][]}
  | {block:'diagram';source:string;kind:string}
  | {block:'code';source:string;language?:string};
export type PageMeta={route:string;title:string;source:string;owner:string;status:string;kind:'contract'|'model'};
export type ReferencePage=PageMeta & {digest?:string;html?:string;links?:string[];headings?:{id:string;title:string;level:number}[];document?:{title:Inline[];blocks:Block[];provenance:{provenance:{source_digest:string;specification_version:string}}};modelRoutes?:Record<string,string>};
export type ReferenceIndex={ess:string;pages:PageMeta[]};
