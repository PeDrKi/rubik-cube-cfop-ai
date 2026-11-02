const {w,ready,}=require('./render_frames');
(async()=>{await ready;const C=w.__cube;const m=require('./render_frames');
 C.setState('LUFUURFRUBDFRDBDFLLURLFDLRURFBLBLDDRDBUBLUBBUBFDLRFRDF');
 const bbox=()=>{const {w:W,h:H,d}=m.ctx.buf;let x0=W,y0=H,x1=-1,y1=-1;for(let y=0;y<H;y++)for(let x=0;x<W;x++)if(d[(y*W+x)*4+3]){if(x<x0)x0=x;if(x>x1)x1=x;if(y<y0)y0=y;if(y>y1)y1=y}
  let area=0;for(let i=0;i<W*H;i++)if(d[i*4+3])area++;return {w:(x1-x0+1)/2,h:(y1-y0+1)/2,area:area/4}};
 const base=(C.clearPose(),bbox());console.log(`khối nghỉ: ${base.w.toFixed(0)}×${base.h.toFixed(0)} px, diện tích ${base.area.toFixed(0)} px²`);
 for(const tok of ['R','U','F',"L'",'M','x','y','z2']){const ar=[],wd=[],ht=[];for(let k=0;k<=1.0001;k+=.1){C.setPose(tok,Math.min(1,k));const b=bbox();ar.push(b.area);wd.push(b.w);ht.push(b.h)}
  const r=a=>(Math.max(...a)/Math.min(...a)).toFixed(3);console.log(`${tok.padEnd(3)} rộng ×${r(wd)}  cao ×${r(ht)}  diện tích ×${r(ar)}  (1.000 = không đổi)`)}
 process.exit(0)})();
