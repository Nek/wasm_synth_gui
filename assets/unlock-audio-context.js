export function unlockAudioContext(cb) {
    const b = document.body;
    const events = ["touchstart", "touchend", "mousedown", "keydown"];
    events.forEach(e => b.addEventListener(e, unlock, false));
    function unlock() {cb(); clean();}
    function clean() {events.forEach(e => b.removeEventListener(e, unlock));}
}