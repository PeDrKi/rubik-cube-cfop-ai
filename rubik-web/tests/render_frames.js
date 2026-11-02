// Xuất ảnh từng khung hình của hiệu ứng xoay: node render_frames.js [trang.html] [trạng_thái_54_ký_tự] [R,F,U2]
const {JSDOM}=require('jsdom'),fs=require('fs'),{Ctx}=require('./fakecanvas');
const page=process.argv[2]||__dirname+'/../static/index.html';
const html=fs.readFileSync(page,'utf8'),SOLVED='UUUUUUUUUDDDDDDDDDFFFFFFFFFBBBBBBBBBLLLLLLLLLRRRRRRRRR';
const stub=async(url,o)=>{const r=url.replace('/api/','').split('?')[0];const ok=d=>({ok:true,json:async()=>d});
 if(r==='solved')return ok({state:SOLVED});if(r==='algos')return ok({oll:[],pll:[]});return {ok:false,json:async()=>({error:'x'})}};
let ctx;
const dom=new JSDOM(html,{url:'http://localhost/',runScripts:'dangerously',pretendToBeVisual:true,beforeParse(w){
 w.fetch=stub;w.devicePixelRatio=2;
 Object.defineProperty(w.HTMLElement.prototype,'clientWidth',{get(){return 640}});Object.defineProperty(w.HTMLElement.prototype,'clientHeight',{get(){return 440}});
 w.HTMLCanvasElement.prototype.getContext=function(){return ctx||(ctx=new Ctx(this))}}});
const w=dom.window;
module.exports={w,ready:new Promise(r=>setTimeout(r,300)),get ctx(){return ctx}};
if(require.main===module){(async()=>{await module.exports.ready;const C=w.__cube;
 const S=process.argv[3]||'LUFUURFRUBDFRDBDFLLURLFDLRURFBLBLDDRDBUBLUBBUBFDLRFRDF';
 const OUT=process.cwd()+'/frames';fs.mkdirSync(OUT,{recursive:true});C.setState(S);ctx.save(OUT+'/_estado.png','#1d222d');
 for(const tok of (process.argv[4]||'R,F').split(',')){for(const k of [0,.3,.5,.7,1]){C.setPose(tok,k);ctx.save(`${OUT}/${tok.replace("'",'p')}_${k}.png`,'#1d222d')}}
 C.clearPose();console.log('Đã ghi ảnh vào '+OUT);process.exit(0)})()}
