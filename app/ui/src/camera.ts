// A small window with the camera's picture, mirrored like Photo Booth, to check the camera works.
import '@fontsource-variable/geist/wght.css';
const root = document.getElementById('root')!;
document.body.style.cssText = 'margin:0;height:100vh;background:#000;color:#f5f5f7;font:13px "Geist Variable",system-ui,sans-serif;display:grid;place-items:center;overflow:hidden';
const say = (t: string) => { root.textContent = t; };
say('Starting the camera…');
navigator.mediaDevices?.getUserMedia({ video: true }).then((stream) => {
  const v = document.createElement('video');
  v.autoplay = true; v.muted = true; v.playsInline = true; v.srcObject = stream;
  v.style.cssText = 'position:fixed;inset:0;width:100%;height:100%;object-fit:cover;transform:scaleX(-1)';
  root.replaceWith(v);
  addEventListener('pagehide', () => stream.getTracks().forEach((t) => t.stop()));
}, (e: Error) => say(e.name === 'NotAllowedError' ? 'Camera access is off. Allow iStash in System Settings › Privacy & Security › Camera.' : e.name === 'NotFoundError' ? 'No camera found.' : `The camera didn’t start: ${e.message}`))
  ?? say('This window can’t use the camera.');
