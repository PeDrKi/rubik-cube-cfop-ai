// Canvas 2D giả: tô đa giác bằng scanline vào bộ đệm RGBA, đủ cho mã vẽ của trang (fill/stroke/lineJoin round).
const {PNG}=require('pngjs'),fs=require('fs');
const hex=c=>{const m=/^#([0-9a-f]{6})$/i.exec(c);if(!m)throw new Error('màu lạ '+c);const n=parseInt(m[1],16);return [n>>16&255,n>>8&255,n&255]};
class Ctx{
 constructor(c){this.c=c;this.fillStyle='#000';this.strokeStyle='#000';this.lineWidth=1;this.lineJoin='miter';this.path=[];this.t=[1,0,0,1,0,0];this.buf=null}
 ensure(){const w=this.c.width,h=this.c.height;if(!this.buf||this.buf.w!==w||this.buf.h!==h)this.buf={w,h,d:Buffer.alloc(w*h*4)}}
 setTransform(a,b,c,d,e,f){this.t=[a,b,c,d,e,f]}
 clearRect(){this.ensure();this.buf.d.fill(0)}
 beginPath(){this.path=[]} moveTo(x,y){this.path.push([[x,y]])} lineTo(x,y){this.path[this.path.length-1].push([x,y])} closePath(){this.path[this.path.length-1].closed=true}
 tx(p){return [this.t[0]*p[0]+this.t[2]*p[1]+this.t[4],this.t[1]*p[0]+this.t[3]*p[1]+this.t[5]]}
 poly(pts,col){this.ensure();const {w,h,d}=this.buf;let y0=Math.max(0,Math.floor(Math.min(...pts.map(p=>p[1]))-1)),y1=Math.min(h-1,Math.ceil(Math.max(...pts.map(p=>p[1]))+1));
  for(let y=y0;y<=y1;y++){const yy=y+.5,xs=[];for(let i=0;i<pts.length;i++){const a=pts[i],b=pts[(i+1)%pts.length];
    if((a[1]>yy)!==(b[1]>yy))xs.push(a[0]+(yy-a[1])*(b[0]-a[0])/(b[1]-a[1]))}
   xs.sort((p,q)=>p-q);for(let k=0;k+1<xs.length;k+=2){for(let x=Math.max(0,Math.round(xs[k]));x<Math.min(w,Math.round(xs[k+1]));x++){const o=(y*w+x)*4;d[o]=col[0];d[o+1]=col[1];d[o+2]=col[2];d[o+3]=255}}}}
 fill(){const col=hex(this.fillStyle);for(const sp of this.path)this.poly(sp.map(p=>this.tx(p)),col)}
 stroke(){const col=hex(this.strokeStyle),hw=this.lineWidth*this.t[0]/2;
  for(const sp of this.path){const P=sp.map(p=>this.tx(p)),n=P.length,segs=sp.closed?n:n-1;
   for(let i=0;i<segs;i++){const a=P[i],b=P[(i+1)%n],dx=b[0]-a[0],dy=b[1]-a[1],L=Math.hypot(dx,dy)||1,nx=-dy/L*hw,ny=dx/L*hw;
    this.poly([[a[0]+nx,a[1]+ny],[b[0]+nx,b[1]+ny],[b[0]-nx,b[1]-ny],[a[0]-nx,a[1]-ny]],col)}
   if(this.lineJoin==='round'&&hw>1)for(const p of P){const c=[];for(let k=0;k<14;k++)c.push([p[0]+hw*Math.cos(k*Math.PI/7),p[1]+hw*Math.sin(k*Math.PI/7)]);this.poly(c,col)}}}
 save(file,bg){this.ensure();const {w,h,d}=this.buf,png=new PNG({width:w,height:h}),B=hex(bg);
  for(let i=0;i<w*h;i++){const a=d[i*4+3]/255;for(let k=0;k<3;k++)png.data[i*4+k]=Math.round(d[i*4+k]*a+B[k]*(1-a));png.data[i*4+3]=255}
  fs.writeFileSync(file,PNG.sync.write(png))}
}
module.exports={Ctx};
