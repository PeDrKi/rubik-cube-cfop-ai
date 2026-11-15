// Chế độ tập trung (Focus): bật/tắt, ẩn UI, popover 2 mép, trả phần tử về chỗ cũ. Dùng: node focus.test.js
const {w,ready}=require('./render_frames');
(async()=>{await ready;let bad=0;const fail=m=>{bad++;console.log('LỖI:',m)};
 const d=w.document,$=i=>d.getElementById(i),body=d.body;
 const key=(c,k)=>w.dispatchEvent(new w.KeyboardEvent('keydown',{code:c,key:k,bubbles:true}));
 const parentOf=s=>d.querySelector(s).parentElement;
 const before={actions:parentOf('.actions'),dock:parentOf('.dock'),winbtns:parentOf('.winbtns'),tr:parentOf('.ovl.tr')};
 const orderBefore=[...d.querySelector('.top').children].map(e=>e.className);
 if(body.classList.contains('focus'))fail('mặc định không được ở chế độ tập trung');
 key('KeyV','v');
 if(!body.classList.contains('focus'))fail('phím V phải bật chế độ tập trung');
 if($('focusM').getAttribute('aria-pressed')!=='true')fail('aria-pressed phải true');
 // phần tử đã được chuyển vào popover
 const inL=s=>$('fpL').contains(d.querySelector(s)),inR=s=>$('fpR').contains(d.querySelector(s));
 if(!inL('.actions')||!inL('.dock'))fail('actions + dock phải ở popover trái');
 if(!inR('.ovl.tr')||!inR('.winbtns')||!inR('.menuwrap'))fail('sơ đồ + nút cửa sổ + menu phải ở popover phải');
 if(!inR('#permbox'))fail('sơ đồ hoán vị phải đi cùng popover phải');
 // popover mặc định đóng
 if(!$('fpL').hidden||!$('fpR').hidden)fail('popover phải đóng khi vừa bật');
 // phím , và . mở popover
 key('Comma',',');if($('fpL').hidden||!$('fpL').classList.contains('open'))fail('phím , phải mở popover trái');
 key('Period','.');if($('fpR').hidden||!$('fpR').classList.contains('open'))fail('phím . phải mở popover phải');
 if($('fpL').classList.contains('open'))fail('mở bên phải phải đóng bên trái');
 key('Escape','Escape');if($('fpR').classList.contains('open'))fail('Esc phải đóng popover đang mở');
 if(!body.classList.contains('focus'))fail('Esc đóng popover không được thoát chế độ tập trung');
 // chuột chạm mép
 $('fzL').dispatchEvent(new w.MouseEvent('mouseenter'));
 if(!$('fpL').classList.contains('open'))fail('chuột chạm mép trái phải mở popover trái');
 $('scene').dispatchEvent(new w.Event('pointerdown'));
 if($('fpL').classList.contains('open'))fail('bấm vào khối phải đóng popover');
 // nút trong popover vẫn hoạt động (Xáo)
 $('fzL').dispatchEvent(new w.MouseEvent('click'));
 $('undo').click();
 // thoát bằng menu trong popover
 $('focusM').click();
 if(body.classList.contains('focus'))fail('mục menu phải thoát chế độ tập trung');
 // đã trả về đúng chỗ + đúng thứ tự
 const after={actions:parentOf('.actions'),dock:parentOf('.dock'),winbtns:parentOf('.winbtns'),tr:parentOf('.ovl.tr')};
 for(const k in before)if(before[k]!==after[k])fail('phần tử '+k+' không trở về chỗ cũ');
 const orderAfter=[...d.querySelector('.top').children].map(e=>e.className);
 if(orderBefore.join('|')!==orderAfter.join('|'))fail('thứ tự trong thanh trên đã đổi: '+orderBefore+' -> '+orderAfter);
 if(d.querySelectorAll('#fpL > *, #fpR > *').length!==2+0&&($('fpL').children.length!==1||$('fpR').children.length!==1))fail('popover phải trống (chỉ còn tiêu đề) sau khi thoát');
 // V trong ô nhập công thức không được bật focus
 $('fx').dispatchEvent(new w.KeyboardEvent('keydown',{code:'KeyV',key:'v',bubbles:true}));
 if(body.classList.contains('focus'))fail('gõ V trong ô nhập công thức không được bật chế độ tập trung');
 // bật/tắt lặp lại nhiều lần không làm nhân đôi hay mất phần tử
 for(let i=0;i<3;i++){key('KeyV','v');key('KeyV','v')}
 if(d.querySelectorAll('.actions').length!==1||d.querySelectorAll('.dock').length!==1||d.querySelectorAll('#net').length!==1)fail('bật/tắt lặp lại làm nhân đôi/mất phần tử');
 if(body.classList.contains('focus'))fail('sau số lần chẵn phải đang tắt');
 console.log(bad?bad+' lỗi':'chế độ tập trung: đạt');process.exit(bad?1:0)})();
