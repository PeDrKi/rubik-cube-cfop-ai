// Cần server đang chạy (cargo run --release -p rubik-web). Dùng: RUBIK_URL=http://127.0.0.1:8080 node semantic.test.js
// So khớp HÌNH ẢNH xoay (mã của trang) với KẾT QUẢ LOGIC của rubik-core (server Rust thật), cho mọi nước đi.
const {w,ready}=require('./render_frames');
const B=(process.env.RUBIK_URL||'http://127.0.0.1:8080')+'/api/';
const post=async(p,b)=>{const r=await fetch(B+p,{method:'POST',body:JSON.stringify(b)});const j=await r.json();if(!r.ok)throw new Error(p+': '+j.error);return j};
(async()=>{await ready;const C=w.__cube;let bad=0,total=0;const rest=C.restCenters();
 const near=(p)=>{let bi=-1,bd=1e9;rest.forEach((q,i)=>{const d=Math.hypot(p[0]-q[0],p[1]-q[1],p[2]-q[2]);if(d<bd){bd=d;bi=i}});return bd<1e-6?bi:-1};
 for(const seed of [42,7]){
  const S=(await post('scramble',{n:25,seed})).state;
  const bases=['U','D','F','B','L','R','M','E','S','x','y','z','u','d','f','b','l','r'];
  for(const b of bases)for(const suf of ['',"'",'2']){const tok=b+suf;total++;
   const want=(await post('apply',{state:S,moves:tok})).state;
   C.setPose(tok,1);const moved=C.stickerCenters();C.clearPose();
   let ok=true;const got=Array(54).fill('?');
   moved.forEach((p,j)=>{const i=near(p);if(i<0){ok=false;return}got[i]=S[j]});
   if(!ok||got.join('')!==want){bad++;console.log('LỆCH',tok,'seed',seed)}}}
 console.log(`${total-bad}/${total} nước đi: hình ảnh xoay KHỚP kết quả của rubik-core (54 sticker mỗi nước)`);process.exit(bad?1:0)})();
