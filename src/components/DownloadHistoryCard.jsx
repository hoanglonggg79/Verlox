import { History, PenLine, Folder } from "lucide-react";

export default function DownloadHistoryCard({ entries, onImport, onFill, onOpen }) {
  const list = entries ?? [];
  return (
    <section className="card" style={{ "--i": 4 }}>
      <div className="card-head">
        <h2>Lịch sử download</h2>
        <button id="btn-import-history" className="btn small" onClick={onImport}>
          <History className="ic" />
          Import JSON cũ
        </button>
      </div>
      <div id="history-list" className="list">
        {entries === null ? null : list.length === 0 ? (
          <div className="empty">Chưa có lịch sử</div>
        ) : (
          list.map((e, i) => (
            <div className="row" key={`${e.hash}-${e.time ?? i}`} style={{ "--i": Math.min(i, 12) }}>
              <span className="hash-value">{e.hash}</span>
              <span className="muted time">{e.time ?? ""}</span>
              <span className="actions">
                <button className="btn small" onClick={() => onFill(e.hash)}>
                  <PenLine className="ic" />
                  Điền hash
                </button>
                {e.path ? (
                  <button className="btn small icon" title="Mở thư mục" onClick={() => onOpen(e.path)}>
                    <Folder className="ic" />
                  </button>
                ) : null}
              </span>
            </div>
          ))
        )}
      </div>
    </section>
  );
}
