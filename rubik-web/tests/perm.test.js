// Sơ đồ hoán vị: hình học 54 ô + khớp với bảng nước đi MT (không cần server). Dùng: node perm.test.js
const {w,ready}=require('./render_frames');
(async()=>{await ready;const P=w.__cube.perm;let bad=0;
 const fail=m=>{bad++;console.log('LỖI:',m)};
 const wrong=P.check();if(wrong.length)fail('nước không khớp bảng MT: '+wrong.join(' '));
 const s=P.slots(),seen=new Set(s.map(q=>q[0].toFixed(2)+','+q[1].toFixed(2)));
 if(s.length!==54||seen.size!==54)fail('54 ô phải có 54 vị trí khác nhau, được '+seen.size);
 const circles=w.document.querySelectorAll('#permsvg circle[fill=none]').length;
 if(circles!==9)fail('phải có 9 vòng tròn, được '+circles);
 // sau một nước, các chấm phải trở về đúng ô và màu theo trạng thái mới
 const st=w.__cube.localMove('UUUUUUUUUDDDDDDDDDFFFFFFFFFBBBBBBBBBLLLLLLLLLRRRRRRRRR',"R'").state;
 w.__cube.setState(st);
 const dots=[...w.document.querySelectorAll('#permsvg circle[stroke="#0a0a0d"]')];
 dots.forEach((d,i)=>{if(Math.abs(+d.getAttribute('cx')-s[i][0])>1e-6||Math.abs(+d.getAttribute('cy')-s[i][1])>1e-6)fail('chấm '+i+' lệch ô');
  if(d.style.fill!=='var(--'+st[i]+')')fail('chấm '+i+' sai màu: '+d.style.fill)});
 // chọn mặt trong sơ đồ 6 mặt: 9 ô của mặt đó sáng (viền trắng, to hơn), các ô khác mờ
 const circ=()=>[...w.document.querySelectorAll('#permsvg circle[stroke-width]')].filter(d=>d.getAttribute('stroke')!==null&&d.getAttribute('fill')===null);
 const dots2=()=>[...w.document.querySelectorAll('#permsvg g')].pop().querySelectorAll('circle');
 const cells=w.document.querySelectorAll('#net i');
 cells[4].click(); // mặt U (ô giữa)
 let on=[...dots2()].map((d,i)=>[i,d.getAttribute('stroke')==='#ffffff']).filter(x=>x[1]).map(x=>x[0]);
 if(on.join()!==[0,1,2,3,4,5,6,7,8].join())fail('chọn mặt U phải sáng ô 0-8, được '+on);
 if([...dots2()][20].style.opacity!=='0.35')fail('ô ngoài mặt đang chọn phải mờ');
 if(+[...dots2()][0].getAttribute('r')<=7.5)fail('ô sáng phải to hơn');
 cells[4].click(); // bấm lại để bỏ chọn
 if([...dots2()].some(d=>d.getAttribute('stroke')==='#ffffff'||d.style.opacity==='0.35'))fail('bỏ chọn mặt phải trả các ô về bình thường');
 // bật/tắt: phím P và mục menu
 const box=w.document.getElementById('permbox'),key=c=>w.dispatchEvent(new w.KeyboardEvent('keydown',{code:'Key'+c,key:c.toLowerCase(),bubbles:true}));
 if(box.hidden)fail('mặc định phải bật');
 key('P');if(!box.hidden)fail('phím P phải tắt sơ đồ');
 key('P');if(box.hidden)fail('phím P phải bật lại sơ đồ');
 w.document.getElementById('permM').click();if(!box.hidden)fail('menu phải tắt sơ đồ');
 if(w.document.getElementById('permM').getAttribute('aria-pressed')!=='false')fail('aria-pressed phải là false khi tắt');
 w.document.getElementById('permM').click();if(box.hidden)fail('menu phải bật lại sơ đồ');
 if(!w.document.querySelector('#net ~ #permbox, .ovl.tr #permbox'))fail('khung phải nằm trong khung sơ đồ 6 mặt');
 console.log(bad?bad+' lỗi':'sơ đồ hoán vị: đạt (54 ô, 9 vòng, mọi nước khớp bảng MT)');process.exit(bad?1:0)})();
