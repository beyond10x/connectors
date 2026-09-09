export type Step={action:string;outcome:string;explanation:string;time:number};
export type Snapshot={scenario:string;clock:number;observation:string;candidate:string;target:string|null;connection:string;viability:string;eligibility:string;admitted:boolean;permission:string;scope:string;custody:boolean;credential_valid:boolean;attempt:string;approval_redemptions:number;provider_sends:number;events:string[];timeline:Step[];error?:string};
type Exports={memory:WebAssembly.Memory;demo_input_reserve:(length:number)=>number;demo_dispatch:()=>number;demo_output_len:()=>number};
export async function openExampleEngine<Request,Response extends {error?:string}>(bytes:BufferSource,dispatchName='demo_dispatch'){
  const {instance}=await WebAssembly.instantiate(bytes,{});
  for(const name of ['demo_input_reserve',dispatchName,'demo_output_len'])if(typeof instance.exports[name]!=='function')throw new Error('The example module is missing its '+name+' interface.');
  if(!(instance.exports.memory instanceof WebAssembly.Memory))throw new Error('The example module exports no memory.');
  const exports=instance.exports as unknown as Exports;
  const encoder=new TextEncoder();const decoder=new TextDecoder('utf-8',{fatal:true});
  const dispatch=instance.exports[dispatchName] as ()=>number;
  return (request:Request):Response=>{
    const input=encoder.encode(JSON.stringify(request));
    if(input.length>4096)throw new Error('Example request exceeds its input limit.');
    const start=exports.demo_input_reserve(input.length);
    new Uint8Array(exports.memory.buffer,start,input.length).set(input);
    const at=dispatch();const len=exports.demo_output_len();
    if(len>1024*1024)throw new Error('Example response exceeds its output limit.');
    const result=JSON.parse(decoder.decode(new Uint8Array(exports.memory.buffer,at,len))) as Response;
    if(result.error)throw new Error(result.error);
    return result;
  };
}
export const openLab=(bytes:BufferSource)=>openExampleEngine<{scenario?:string;action?:string},Snapshot>(bytes);
