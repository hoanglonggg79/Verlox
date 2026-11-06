import { useEffect, useRef } from "react";

function ts(t) {
  return new Date(t).toLocaleTimeString("vi-VN", { hour12: false });
}

export default function LogPanel({ logs }) {
  const ref = useRef(null);

  useEffect(() => {
    const el = ref.current;
    if (el) el.scrollTop = el.scrollHeight;
  }, [logs]);

  return (
    <section className="term-section logs-full" style={{ "--i": 0 }}>
      <div className="term">
        <div id="log" className="log" ref={ref}>
          {logs.length === 0 ? (
            <div className="log-line muted">Chưa có log nào — tương tác với app để thấy log tại đây.</div>
          ) : null}
          {logs.map((l) => (
            <div key={l.id} className={`log-line ${l.kind}`}>
              [{ts(l.t)}] {l.message}
            </div>
          ))}
          <span className="term-cursor" aria-hidden="true"></span>
        </div>
      </div>
    </section>
  );
}
