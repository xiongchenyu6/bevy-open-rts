// Settle touch intent before emitting a mouse click: a drag never selects/orders.
export function installTouchControls(canvas) {
  const pointers = new Map(); let gesture = null, sequence = Promise.resolve();
  window.rtsTouchCamera = {x:0,y:0,zoom:0};
  const ui = () => document.querySelector('dialog[open]');
  const emit = (type,p,button) => canvas.dispatchEvent(new PointerEvent(type,{bubbles:true,cancelable:true,pointerType:'mouse',pointerId:999,isPrimary:true,clientX:p.x,clientY:p.y,button,buttons:type==='pointerup'?0:button===2?2:1}));
  function click(p,button) { sequence=sequence.then(async()=> { emit('pointermove',p,-1); emit('pointerdown',p,button); await new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r))); emit('pointerup',p,button); }); }
  canvas.addEventListener('contextmenu',e=>e.preventDefault());
  for(const type of ['touchstart','touchmove','touchend','touchcancel']) canvas.addEventListener(type,e=>{e.preventDefault();e.stopImmediatePropagation();},{capture:true,passive:false});
  for(const type of ['pointerdown','pointermove','pointerup','pointercancel']) canvas.addEventListener(type,e=>{
    if(e.pointerType!=='touch')return;
    e.preventDefault();e.stopImmediatePropagation();
    if(ui())return;
    const point={x:e.clientX,y:e.clientY};
    const bounds=canvas.getBoundingClientRect();
    const local={x:point.x-bounds.left,y:point.y-bounds.top};
    if(type==='pointerdown') {
      pointers.set(e.pointerId,point);canvas.setPointerCapture(e.pointerId);
      if(pointers.size===1)gesture={start:point,last:point,time:performance.now(),drag:false,pinch:null,hud:window.rtsTouchWorldEnabled===false||local.y<76||(window.rtsTouchHudRects||[]).some(r=>local.x>=r[0]&&local.y>=r[1]&&local.x<=r[2]&&local.y<=r[3])};
      else if(gesture) {gesture.drag=true;gesture.pinch=null;}
      return;
    }
    if(!gesture||!pointers.has(e.pointerId))return;
    if(type==='pointermove') {
      pointers.set(e.pointerId,point);
      const match=canvas.dataset.appScreen==='InMatch';
      if(match&&gesture.hud){if(Math.hypot(point.x-gesture.start.x,point.y-gesture.start.y)>12)gesture.drag=true;gesture.last=point;return;}
      if(pointers.size===2){const [a,b]=[...pointers.values()];const distance=Math.hypot(a.x-b.x,a.y-b.y);if(match&&gesture.pinch)window.rtsTouchCamera.zoom+=(gesture.pinch-distance)*0.012;gesture.pinch=distance;gesture.drag=true;}
      else {
        if(Math.hypot(point.x-gesture.start.x,point.y-gesture.start.y)>12)gesture.drag=true;
        if(!match&&gesture.drag)canvas.dispatchEvent(new WheelEvent('wheel',{bubbles:true,cancelable:true,deltaY:(gesture.last.y-point.y)/2.4,deltaMode:0,clientX:point.x,clientY:point.y}));
        if(match&&gesture.drag){window.rtsTouchCamera.x+=point.x-gesture.last.x;window.rtsTouchCamera.y+=point.y-gesture.last.y;}
      }
      gesture.last=point;return;
    }
    pointers.delete(e.pointerId);
    if(type==='pointerup'&&!gesture.drag&&pointers.size===0)click(point,performance.now()-gesture.time>500&&!gesture.hud&&canvas.dataset.appScreen==='InMatch'?2:0);
    if(!pointers.size)gesture=null;
  },{capture:true,passive:false});
}
