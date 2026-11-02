const {JSDOM}=require('jsdom');const fs=require('fs');
const html=fs.readFileSync(__dirname+'/../static/index.html','utf8');
const SOLVED='UUUUUUUUUDDDDDDDDDFFFFFFFFFBBBBBBBBBLLLLLLLLLRRRRRRRRR';
const log=[];let hintN=0;
const fetchStub=async(url,opt)=>{
  const route=url.replace('/api/','').split('?')[0];let body={};try{body=JSON.parse(opt&&opt.body||'{}')}catch(e){}
  log.push([route,body]);const ok=d=>({ok:true,json:async()=>d});
  if(route==='solved')return ok({state:SOLVED});
  if(route==='algos')return ok({oll:[{name:'OLL 1',alg:'R U R'}],pll:[{name:'Aa',alg:'x R'}]});
  if(route==='apply')return ok({state:SOLVED,moves:body.moves.split(' '),solved:true});
  if(route==='hint'){hintN++;return ok(hintN<=2?{label:'Cross',moves:['R','U',"R'"],stage:'cross',case_name:null}:{label:'F2L',moves:['F'],stage:'f2l',case_name:null})}
  if(route==='auth/change_password')return body.old_password==='good'?ok({}):{ok:false,status:401,json:async()=>({error:'Mật khẩu hiện tại không đúng'})};
  if(route==='auth/delete')return body.password==='good'?ok({}):{ok:false,status:401,json:async()=>({error:'Mật khẩu hiện tại không đúng'})};
  if(route==='leaderboard'||route==='profile')return ok(route==='profile'?{username:'lien',games:0,solves:0,best_time_ms:null,best_moves:null,avg_time_ms:null,recent:[]}:{rows:[]});
  return {ok:false,json:async()=>({error:'x'})}};
const dom=new JSDOM(html,{url:'http://localhost/',runScripts:'dangerously',pretendToBeVisual:true,beforeParse(w){w.fetch=fetchStub}});
const w=dom.window,d=w.document,sleep=ms=>new Promise(r=>setTimeout(r,ms));
const key=(type,k,code,extra={})=>d.dispatchEvent(new w.KeyboardEvent(type,Object.assign({key:k,code:code||k,bubbles:true},extra)));
let pOff=0;const reset=()=>{log.length=0;pOff=w.__cube.played.length};
const applies=()=>w.__cube.played.slice(pOff);
(async()=>{
  await sleep(200);d.getElementById('spd').value=30;
  let pass=0,fail=0;const t=(n,c)=>{console.log((c?'ĐẠT ':'LỖI ')+n);c?pass++:fail++};
  const C=w.__cube;
  t('mô hình khối: 26 cubie + 54 sticker',C.cubies.length===26&&C.stickers.filter(Boolean).length===54);
  const cnt={};d.querySelectorAll('#net i').forEach(e=>{const m=(e.style.background||'').match(/--([UDFBLR])/);if(m)cnt[m[1]]=(cnt[m[1]]||0)+1});
  t('sơ đồ 6 mặt tô đúng 9 ô mỗi màu: '+JSON.stringify(cnt),'UDFBLR'.split('').every(k=>cnt[k]===9));
  // H: chỉ tính + hiển thị, KHÔNG tự chạy
  key('keydown','h','KeyH');key('keyup','h','KeyH');await sleep(150);
  t('H hiển thị gợi ý nhưng chưa chạy nước nào',!!d.getElementById('hrun')&&applies().length===0);
  // Ctrl bấm rồi thả => chạy gợi ý
  key('keydown','Control','ControlLeft',{ctrlKey:true});key('keyup','Control','ControlLeft');await sleep(700);
  t("Ctrl bấm rồi thả chạy đủ gợi ý R U R'",JSON.stringify(applies())===JSON.stringify(['R','U',"R'"]));
  // Ctrl + Z không được tính là Ctrl đơn
  reset();key('keydown','h','KeyH');await sleep(150);reset();
  key('keydown','Control','ControlLeft',{ctrlKey:true});key('keydown','z','KeyZ',{ctrlKey:true});key('keyup','z','KeyZ');key('keyup','Control','ControlLeft');await sleep(300);
  t('Ctrl+Z không kích hoạt chạy gợi ý',applies().length===0);
  // Alt đơn: chạy từng nước, hết thì tự tính gợi ý kế tiếp
  reset();hintN=0;key('keydown','h','KeyH');await sleep(150);reset();
  key('keydown','Alt','AltLeft',{altKey:true});key('keyup','Alt','AltLeft');await sleep(300);
  t('Alt đơn lần 1 chạy đúng 1 nước (R)',JSON.stringify(applies())===JSON.stringify(['R']));
  key('keydown','Alt','AltLeft',{altKey:true});key('keyup','Alt','AltLeft');await sleep(300);
  key('keydown','Alt','AltLeft',{altKey:true});key('keyup','Alt','AltLeft');await sleep(500);
  t("Alt đơn lần 2,3 chạy U rồi R'",JSON.stringify(applies())===JSON.stringify(['R','U',"R'"]));
  t('hết nước thì tự gọi tính gợi ý kế tiếp',log.filter(l=>l[0]==='hint').length>=1);
  // Alt + phím mặt = nước đúp, không kích hoạt Alt đơn
  reset();key('keydown','Alt','AltLeft',{altKey:true});key('keydown','u','KeyU',{altKey:true});key('keyup','u','KeyU');key('keyup','Alt','AltLeft');await sleep(300);
  t('Alt+U chạy U2 và không chạy thêm gợi ý',JSON.stringify(applies())===JSON.stringify(['U2']));
  // Shift = nghịch, Ctrl+phím = wide, M/E/S, x/y/z
  reset();key('keydown','R','KeyR',{shiftKey:true});await sleep(150);key('keydown','r','KeyR',{ctrlKey:true});await sleep(150);key('keydown','m','KeyM');await sleep(150);key('keydown','x','KeyX',{shiftKey:true});await sleep(200);
  t("Shift+R=R', Ctrl+R=r, M=M, Shift+X=x'",JSON.stringify(applies())===JSON.stringify(["R'",'r','M',"x'"]));
  // Ctrl+Y mở thống kê, ghi nhật ký 1 lần/ván
  d.getElementById('sdlg').showModal=function(){this.setAttribute('open','')};d.getElementById('sdlg').close=function(){this.removeAttribute('open')};
  w.localStorage.setItem('rubik_oll_pll_history',JSON.stringify([[1,'OLL','OLL 1',9],[2,'OLL','OLL 1',8],[3,'PLL','Aa',12]]));
  key('keydown','y','KeyY',{ctrlKey:true});await sleep(50);
  const info=d.getElementById('sinfo').textContent;t('thống kê đếm đúng (OLL: 1/1 thế, 2 lần): '+info,info.includes('1/1')&&info.includes('2 lần'));
  // nhập TSV: chống trùng + bỏ dòng sai
  const tsv='epoch_giay\tthoi_gian_utc\tloai\tten_the\tso_nuoc\n1\tx\tOLL\tOLL 1\t9\n99\tx\tOLL\tOLL 1\t7\nrac\n';
  const file={text:async()=>tsv};const inp=d.getElementById('simpf');Object.defineProperty(inp,'files',{value:[file],configurable:true});
  await inp.onchange({target:inp});await sleep(50);
  const h=JSON.parse(w.localStorage.getItem('rubik_oll_pll_history'));t('nhập TSV: bỏ trùng và dòng lỗi, thêm đúng 1 dòng mới (tổng 4)',h.length===4);
  d.getElementById('sdlg').close();   // đóng hộp thoại thống kê, nếu không phím tắt bị chặn
  // ---- Tính nước đi tại chỗ: không gọi server, kết quả khớp engine Rust ----
  const SOLVED_S='UUUUUUUUUDDDDDDDDDFFFFFFFFFBBBBBBBBBLLLLLLLLLRRRRRRRRR';
  t('nước R từ khối giải khớp vectơ chuẩn của Rust',C.localMove(SOLVED_S,'R').state==='UUFUUFUUFDDBDDBDDBFFDFFDFFDUBBUBBUBBLLLLLLLLLRRRRRRRRR');
  let mixS=SOLVED_S;for(const m of "R U2 F' r M' x z2 d b'".split(' '))mixS=C.localMove(mixS,m).state;
  t('chuỗi hỗn hợp (mặt, 2 lớp, M, x, z2) khớp vectơ chuẩn của Rust',mixS==='LFBDDLUDBLBBUULRDRFRDDRRDRRDRULLUFLFBBLBBFUFFLURUFFUBD');
  const ords=[];for(const b of 'UDFBLRMESxyzudfblr'){let x=SOLVED_S,n=0;do{x=C.localMove(x,b).state;n++}while(x!==SOLVED_S&&n<5);ords.push(n)}
  t('mỗi nước cơ bản lặp 4 lần thì về nguyên trạng',ords.every(n=>n===4));
  t("X2' dùng chung bảng với X2",C.localMove(SOLVED_S,"R2'").state===C.localMove(SOLVED_S,'R2').state&&C.localMove(SOLVED_S,'R2').state!==SOLVED_S);
  t('ký hiệu lạ trả về null (để hỏi server)',C.localMove(SOLVED_S,'Q')===null);
  {const sx=Array(6).fill("R U R' U'").join(' ').split(' ');let x=SOLVED_S;for(const m of sx)x=C.localMove(x,m).state;t("(R U R' U') lặp 6 lần về nguyên trạng",x===SOLVED_S)}
  // ---- Bàn phím: menu "Thêm" và cửa sổ nổi ----
  {const mb=d.getElementById('menub'),menu=d.getElementById('menu'),kd=(el,k,extra={})=>el.dispatchEvent(new w.KeyboardEvent('keydown',Object.assign({key:k,code:k,bubbles:true,cancelable:true},extra)));
   const items=[...menu.querySelectorAll('[role=menuitem]')];
   mb.focus();kd(mb,'ArrowDown');
   t('↓ trên nút Thêm mở menu và đưa con trỏ vào mục đầu',menu.classList.contains('open')&&d.activeElement===items[0]&&mb.getAttribute('aria-expanded')==='true');
   kd(d.activeElement,'ArrowDown');t('↓ chuyển sang mục kế',d.activeElement===items[1]);
   kd(d.activeElement,'End');t('End nhảy tới mục cuối',d.activeElement===items[items.length-1]);
   kd(d.activeElement,'ArrowDown');t('↓ ở mục cuối quay vòng về mục đầu',d.activeElement===items[0]);
   kd(d.activeElement,'ArrowUp');t('↑ ở mục đầu quay vòng xuống mục cuối',d.activeElement===items[items.length-1]);
   kd(d.activeElement,'Home');t('Home nhảy về mục đầu',d.activeElement===items[0]);
   reset();kd(d.activeElement,'ArrowDown');await sleep(120);
   t('mũi tên trong menu không xoay mặt khối',applies().length===0);
   kd(d.activeElement,'Escape');
   t('Esc đóng menu và trả con trỏ về nút Thêm',!menu.classList.contains('open')&&d.activeElement===mb&&mb.getAttribute('aria-expanded')==='false');
   kd(mb,'ArrowUp');t('↑ trên nút Thêm mở menu và vào mục cuối',menu.classList.contains('open')&&d.activeElement===items[items.length-1]);
   kd(d.activeElement,'Tab');t('Tab đóng menu',!menu.classList.contains('open'));
   mb.focus();}
  {const win=d.getElementById('win-lib'),btn=d.getElementById('winLibBtn'),kd=(el,k,extra={})=>{const ev=new w.KeyboardEvent('keydown',Object.assign({key:k,code:k,bubbles:true,cancelable:true},extra));el.dispatchEvent(ev);return ev};
   if(!win.hidden)btn.click();
   btn.focus();btn.click();
   t('mở cửa sổ bằng nút: con trỏ vào trong cửa sổ',!win.hidden&&win.contains(d.activeElement));
   const fs=[...win.querySelectorAll('button,input,a[href],select,textarea,[tabindex]:not([tabindex="-1"])')].filter(n=>!n.closest('[hidden]'));
   fs[fs.length-1].focus();const ev=kd(fs[fs.length-1],'Tab');
   t('Tab ở phần tử cuối nhảy về phần tử đầu (con trỏ không thoát ra ngoài)',ev.defaultPrevented&&d.activeElement===fs[0]);
   const ev2=kd(fs[0],'Tab',{shiftKey:true});
   t('Shift+Tab ở phần tử đầu nhảy xuống phần tử cuối',ev2.defaultPrevented&&d.activeElement===fs[fs.length-1]);
   kd(d.activeElement,'Escape');
   t('Esc đóng cửa sổ và trả con trỏ về nút đã mở nó',win.hidden&&d.activeElement===btn);
   // cửa sổ tự mở vì gợi ý (H) không được cướp con trỏ
   d.body.focus();key('keydown','h','KeyH');await sleep(200);
   t('gợi ý tự mở bảng công thức nhưng không cướp con trỏ bàn phím',!d.getElementById('win-plan').hidden&&!d.getElementById('win-plan').contains(d.activeElement));
   if(!win.hidden)btn.click()}
  // Enter trên nút đang được chọn bằng bàn phím phải kích hoạt nút đó, không bị phím tắt "Giải tự động" nuốt
  {const b=d.getElementById('undo');b.matches=sel=>sel===':focus-visible'||Element.prototype.matches.call(b,sel);let clicked=0;b.addEventListener('click',()=>clicked++);b.focus();
   const ev=new w.KeyboardEvent('keydown',{key:'Enter',code:'Enter',bubbles:true,cancelable:true});b.dispatchEvent(ev);
   t('Enter trên nút đang focus bằng bàn phím: phím tắt không chặn hành vi mặc định',!ev.defaultPrevented)}
  // ---- Đổi mật khẩu / xoá tài khoản ----
  {for(const id of ['pwdlg','lbdlg']){const dl=d.getElementById(id);dl.showModal=function(){this.setAttribute('open','')};dl.close=function(){this.removeAttribute('open')}}
   const $d=id=>d.getElementById(id),submit=()=>$d('pwform').dispatchEvent(new w.Event('submit',{bubbles:true,cancelable:true})),calls=r=>log.filter(l=>l[0]===r);
   w.setMe(null);await w.openBoard('time');
   t('chưa đăng nhập: không có nút đổi mật khẩu / xoá tài khoản',$d('chpw').hidden&&$d('delacc').hidden);
   w.setMe({id:1,username:'lien'});await w.openBoard('time');
   t('đã đăng nhập nhưng ở tab khác: các nút vẫn ẩn',$d('chpw').hidden&&$d('delacc').hidden);
   await w.openBoard('me');await sleep(50);
   t('tab "Của tôi": hiện nút đổi mật khẩu và xoá tài khoản',!$d('chpw').hidden&&!$d('delacc').hidden);
   $d('chpw').click();
   t('mở hộp thoại đổi mật khẩu, ô mật khẩu mới hiện, cảnh báo xoá ẩn',$d('pwdlg').open&&!$d('pwnewrow').hidden&&$d('pwwarn').hidden&&$d('pwuser').value==='lien');
   $d('pwold').value='good';$d('pwnew').value='matkhau2';$d('pwnew2').value='khac';submit();await sleep(50);
   t('hai mật khẩu mới không khớp: báo lỗi, không gọi server',$d('pwerr').textContent.includes('không khớp')&&calls('auth/change_password').length===0&&$d('pwdlg').open);
   $d('pwnew2').value='matkhau2';submit();await sleep(80);
   const cp=calls('auth/change_password');
   t('đổi mật khẩu đúng: gọi server với mật khẩu cũ/mới, đóng hộp thoại, xoá ô nhập',cp.length===1&&cp[0][1].old_password==='good'&&cp[0][1].new_password==='matkhau2'&&!$d('pwdlg').open&&$d('pwold').value==='');
   $d('chpw').click();$d('pwold').value='sai';$d('pwnew').value=$d('pwnew2').value='matkhau3';submit();await sleep(80);
   t('sai mật khẩu hiện tại: hiện lỗi của server, hộp thoại vẫn mở',$d('pwerr').textContent.includes('không đúng')&&$d('pwdlg').open);
   $d('pwclose').click();
   $d('delacc').click();
   t('hộp thoại xoá tài khoản: có cảnh báo, ẩn ô mật khẩu mới',$d('pwdlg').open&&$d('pwnewrow').hidden&&!$d('pwwarn').hidden&&$d('pwsub').classList.contains('danger'));
   $d('pwold').value='sai';submit();await sleep(80);
   t('xoá với mật khẩu sai: báo lỗi, vẫn đăng nhập',$d('pwerr').textContent.includes('không đúng')&&$d('accName').textContent==='lien');
   $d('pwold').value='good';submit();await sleep(80);
   t('xoá với mật khẩu đúng: đăng xuất, đóng cả hai hộp thoại',calls('auth/delete').length===2&&$d('accName').textContent==='Đăng nhập'&&!$d('pwdlg').open&&!$d('lbdlg').open);
  }
  // ---- Trợ năng khối 3D: mô tả bằng lời + thông báo sau khi xoay ----
  {const cvEl=d.getElementById('cv'),live=d.getElementById('a11y');
   C.setState(SOLVED_S);
   t('canvas có role=img và mô tả "đã giải xong" khi khối nguyên vẹn',cvEl.getAttribute('role')==='img'&&cvEl.getAttribute('aria-label')==='Khối Rubik đã giải xong.');
   C.setState(C.localMove(SOLVED_S,'R').state);
   const lab=cvEl.getAttribute('aria-label');
   t('khối xáo: mô tả đủ 6 mặt và đúng màu (mặt phải/trước...)',lab.startsWith('Khối Rubik chưa giải.')&&['trên','dưới','trước','sau','trái','phải'].every(f=>lab.includes('Mặt '+f+' '))&&lab.includes('Mặt trước (F): hàng 1 xanh lá, xanh lá, trắng'));
   C.setState(SOLVED_S);
   key('keydown','r','KeyR');await sleep(300);
   t('sau khi xoay xong, vùng aria-live thông báo nước đã đi',live.getAttribute('aria-live')==='polite'&&/Đã xoay: R\./.test(live.textContent));
   key('keydown','R','KeyR',{shiftKey:true});await sleep(300);
   t('xoay về nguyên trạng: thông báo thêm "Khối đã giải xong"',/Đã xoay: R'\. Khối đã giải xong\./.test(live.textContent));
   t('sơ đồ 6 mặt trùng lặp được ẩn khỏi trình đọc màn hình',d.getElementById('net').getAttribute('aria-hidden')==='true')}
  // ---- Hiệu ứng xoay lớp (bộ dựng canvas): phải là XOAY quanh trục, ô đi trên cung tròn ----
  const R0=C.centers().map(p=>Math.hypot(...p));
  const at=(tok,k)=>{C.setPose(tok,k);return {deg:C.anim.deg,ax:C.anim.ax,c:C.centers()}};
  const ks=[0.1,0.25,0.5,0.75,0.9,1];
  const seq=ks.map(k=>at('R',k));
  t('R: trục xoay là +X, góc đi từ 0 tới −90° và tăng dần đều: '+seq.map(x=>x.deg.toFixed(0)).join(' → '),
    seq.every(x=>x.ax.join()==='1,0,0'&&x.deg<0&&x.deg>=-90)&&seq.every((x,i)=>i===0||x.deg<seq[i-1].deg)&&Math.abs(seq[5].deg+90)<1e-9);
  t('mọi ô giữ nguyên bán kính quỹ đạo ở mọi góc (đi theo cung tròn, không thu phóng)',
    ['R',"R'",'F','U2','M','x','u'].every(tok=>ks.every(k=>at(tok,k).c.every((p,i)=>Math.abs(Math.hypot(...p)-R0[i])<1e-9))));
  C.clearPose();const rest=C.centers();const moved=at('R',0.5).c.filter((p,i)=>Math.hypot(p[0]-rest[i][0],p[1]-rest[i][1],p[2]-rest[i][2])>1e-9).length;
  t('R: 8 ô dịch chuyển (ô tâm trên trục thì đứng yên), các ô ngoài lớp R không nhúc nhích',moved===8&&at('R',0.5).c.every((p,i)=>rest[i][0]>0.5||Math.hypot(p[0]-rest[i][0],p[1]-rest[i][1],p[2]-rest[i][2])<1e-9));
  t("R' xoay ngược chiều R (góc dương)",at("R'",0.5).deg>0&&at('R',0.5).deg<0);
  t('nước đúp R2 kết thúc ở 180°',Math.abs(at('R2',1).deg+180)<1e-9);
  t('kích thước lớp: R=9, u=17, M=8, x=26',C.layerSize('R')===9&&C.layerSize('u')===17&&C.layerSize('M')===8&&C.layerSize('x')===26);
  C.clearPose();
  // Chạy thật bằng phím: góc phải thay đổi liên tục theo thời gian rồi trả về trạng thái nghỉ
  d.getElementById('spd').value=400;key('keydown','R','KeyR');
  const live=[];for(let n=0;n<5;n++){await sleep(80);live.push(C.anim?C.anim.deg:null)}
  await sleep(450);
  const act=live.filter(x=>x!==null);
  t('xoay bằng phím R: góc đổi liên tục theo thời gian ['+act.map(x=>x.toFixed(0)).join(', ')+'] rồi dừng',act.length>=3&&act.every((x,i)=>i===0||x<=act[i-1])&&C.anim===null);
  d.getElementById('spd').value=30;
  // ---- Cửa sổ nổi (popover): mở, kéo, đổi kích thước, lưu vị trí, thứ tự lớp ----
  const WP=d.getElementById('win-plan'),WL=d.getElementById('win-lib'),num=parseFloat;
  const geo=el=>({x:num(el.style.left),y:num(el.style.top),w:num(el.style.width),h:num(el.style.height)});
  const E=w.PointerEvent||w.MouseEvent;
  const P=(type,x,y,target)=>(target||d).dispatchEvent(new E(type,{clientX:x,clientY:y,bubbles:true,button:0}));
  const drag=(el,from,to)=>{P('pointerdown',from[0],from[1],el);P('pointermove',to[0],to[1]);P('pointerup',to[0],to[1])};
  [WP,WL].forEach(el=>{if(!el.hidden)el.querySelector('[data-wact=close]').click()});   // các test phím H ở trên đã tự mở bảng
  t('đóng bằng nút ✕: cả hai cửa sổ nổi đều ẩn',WP.hidden&&WL.hidden);
  d.getElementById('winPlanBtn').click();
  t('bấm nút "Bảng" mở cửa sổ và nút sáng lên',!WP.hidden&&d.getElementById('winPlanBtn').classList.contains('on'));
  let g=geo(WP);t(`vị trí mặc định nằm gọn trong màn hình (${JSON.stringify(g)})`,g.x>0&&g.y>=0&&g.x+g.w<=1024&&g.y+g.h<=768);
  const head=WP.querySelector('.wh');
  // kéo tiêu đề
  let g0=geo(WP);drag(head,[g0.x+60,g0.y+15],[g0.x+60-120,g0.y+15+90]);let g1=geo(WP);
  t(`kéo tiêu đề dịch đúng (−120, +90): thực tế (${g1.x-g0.x}, ${g1.y-g0.y}), kích thước giữ nguyên`,g1.x-g0.x===-120&&g1.y-g0.y===90&&g1.w===g0.w&&g1.h===g0.h);
  // kéo ra khỏi màn hình: vẫn giữ lại phần tiêu đề để kéo về
  drag(head,[g1.x+60,g1.y+15],[-9000,-9000]);let g2=geo(WP);
  t('kéo ra ngoài màn hình: vẫn còn ≥120px tiêu đề nhìn thấy và y ≥ 0',g2.x+g2.w>=120&&g2.y>=0&&g2.x<=1024-120);
  drag(head,[g2.x+130,g2.y+15],[9000,9000]);g2=geo(WP);
  t('kéo về phía góc dưới-phải: không văng khỏi màn hình',g2.x<=1024-120&&g2.y<=768-44);
  // đặt về vị trí xác định (200,150) để thử đổi kích thước
  drag(head,[g2.x+150,g2.y+15],[200+150,150+15]);let b=geo(WP);t(`đặt cửa sổ về (200,150): (${b.x}, ${b.y})`,b.x===200&&b.y===150);
  const H=dir=>WP.querySelector(`.rz[data-dir=${dir}]`);
  drag(H('se'),[b.x+b.w-3,b.y+b.h-3],[b.x+b.w-3+60,b.y+b.h-3+40]);let c=geo(WP);
  t(`kéo góc dưới-phải: rộng +60, cao +40 (thực tế ${c.w-b.w}, ${c.h-b.h}), vị trí giữ nguyên`,c.w-b.w===60&&c.h-b.h===40&&c.x===b.x&&c.y===b.y);
  drag(H('e'),[c.x+c.w-2,c.y+100],[c.x+c.w-2-30,c.y+100]);let e1=geo(WP);t(`kéo cạnh phải sang trái 30px: rộng −30 (thực tế ${e1.w-c.w})`,e1.w-c.w===-30&&e1.h===c.h&&e1.x===c.x);
  drag(H('w'),[e1.x+2,e1.y+100],[e1.x+2-50,e1.y+100]);let w1=geo(WP);t(`kéo cạnh trái sang trái 50px: rộng +50 và x −50 (thực tế ${w1.w-e1.w}, ${w1.x-e1.x})`,w1.w-e1.w===50&&w1.x-e1.x===-50&&w1.y===e1.y&&w1.h===e1.h);
  drag(H('n'),[w1.x+100,w1.y+2],[w1.x+100,w1.y+2-30]);let n1=geo(WP);t(`kéo cạnh trên lên 30px: cao +30 và y −30 (thực tế ${n1.h-w1.h}, ${n1.y-w1.y})`,n1.h-w1.h===30&&n1.y-w1.y===-30&&n1.w===w1.w);
  drag(H('nw'),[n1.x+3,n1.y+3],[n1.x+3+20,n1.y+3+10]);let nw=geo(WP);t(`kéo góc trên-trái vào trong (20,10): rộng −20, cao −10, x +20, y +10`,nw.w-n1.w===-20&&nw.h-n1.h===-10&&nw.x-n1.x===20&&nw.y-n1.y===10);
  let q0=geo(WP);drag(H('se'),[q0.x+q0.w-3,q0.y+q0.h-3],[-9000,-9000]);let mn=geo(WP);
  t(`thu nhỏ hết cỡ: dừng ở kích thước tối thiểu 280×150 (thực tế ${mn.w}×${mn.h})`,mn.w===280&&mn.h===150&&mn.x===q0.x&&mn.y===q0.y);
  drag(H('se'),[mn.x+mn.w-3,mn.y+mn.h-3],[mn.x+mn.w-3+9000,mn.y+mn.h-3+9000]);let mx=geo(WP);
  t(`phóng to hết cỡ: không vượt màn hình (thực tế ${mx.w}×${mx.h})`,mx.w<=1024-16&&mx.h<=768-16);
  // lưu vị trí
  const saved=JSON.parse(w.localStorage.getItem('rubik_windows_v1')||'{}').plan;
  t('vị trí và kích thước được lưu vào localStorage',saved&&saved.x===mx.x&&saved.y===mx.y&&saved.w===mx.w&&saved.h===mx.h);
  // thứ tự lớp
  d.getElementById('winLibBtn').click();
  const z=el=>+el.style.zIndex;
  t('mở cửa sổ thứ hai: nằm trên cửa sổ đầu',!WL.hidden&&z(WL)>z(WP)&&WL.classList.contains('focus')&&!WP.classList.contains('focus'));
  P('pointerdown',100,100,WP.querySelector('.wb'));
  t('bấm vào cửa sổ dưới: nó nổi lên trên cùng',z(WP)>z(WL)&&WP.classList.contains('focus')&&!WL.classList.contains('focus'));
  // thu gọn
  const gm=geo(WP);head.dispatchEvent(new w.MouseEvent('dblclick',{bubbles:true}));
  t('nhấp đúp tiêu đề thu gọn cửa sổ',WP.classList.contains('min'));
  head.dispatchEvent(new w.MouseEvent('dblclick',{bubbles:true}));
  t('nhấp đúp lần nữa mở rộng lại, kích thước giữ nguyên',!WP.classList.contains('min')&&geo(WP).h===gm.h);
  WP.querySelector('[data-wact=min]').click();t('nút "–" cũng thu gọn',WP.classList.contains('min'));WP.querySelector('[data-wact=min]').click();
  // đóng
  WP.querySelector('[data-wact=close]').click();
  t('nút ✕ đóng cửa sổ và tắt đèn nút',WP.hidden&&!d.getElementById('winPlanBtn').classList.contains('on'));
  // tự mở khi có kết quả; giữ vị trí đã lưu
  d.getElementById('spd').value=30;key('keydown','h','KeyH');await sleep(200);
  t('xin gợi ý (H) tự mở bảng công thức, giữ đúng vị trí đã kéo',!WP.hidden&&!!d.getElementById('hrun')&&geo(WP).x===mx.x&&geo(WP).w===mx.w);
  // Thu gọn rồi có kết quả mới thì tự bung ra
  WP.querySelector('[data-wact=min]').click();hintN=0;await sleep(700);key('keydown','h','KeyH');await sleep(250);
  t('đang thu gọn mà có gợi ý mới thì tự bung ra',!WP.classList.contains('min'));
  // Esc và Ctrl+J
  WL.querySelector('[data-wact=close]').click();
  key('keydown','j','KeyJ',{ctrlKey:true});
  t('Ctrl+J mở thư viện và đưa con trỏ vào ô tìm kiếm',!WL.hidden&&d.activeElement===d.getElementById('q'));
  d.getElementById('q').dispatchEvent(new w.KeyboardEvent('keydown',{key:'Escape',code:'Escape',bubbles:true}));
  t('Esc khi đang ở trong cửa sổ thì đóng cửa sổ đó (kể cả đang gõ ô tìm kiếm)',WL.hidden&&!WP.hidden);
  // thư viện: tìm kiếm và nạp công thức
  d.getElementById('winLibBtn').click();
  d.getElementById('q').value='oll 1';d.getElementById('q').dispatchEvent(new w.Event('input',{bubbles:true}));
  const rows=d.querySelectorAll('#lib [data-alg]');t('tìm "oll 1" ra đúng 1 dòng trong cửa sổ thư viện',rows.length===1);
  rows[0].dispatchEvent(new w.MouseEvent('click',{bubbles:true}));
  t('bấm dòng công thức nạp vào ô nhập',d.getElementById('fx').value===rows[0].dataset.alg&&rows[0].dataset.alg.length>3);
  console.log(`\n${pass} đạt, ${fail} lỗi`);process.exit(fail?1:0)
})();
