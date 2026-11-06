import { useEffect, useRef, useState } from "react";
import { Broom, ExternalLink, Folder, Link, Play, RefreshCw, Trash2, Unlink } from "lucide-react";
import { fmtBytes } from "../lib/utils.js";

function SkeletonRows() {
  return (
    <>
      {[0, 1, 2].map((i) => (
        <div className="row skeleton" key={i} aria-hidden="true">
          <span className="sk" style={{ width: "38%" }}></span>
          <span className="sk" style={{ width: "14%" }}></span>
          <span className="sk" style={{ width: "10%" }}></span>
          <span className="sk" style={{ width: "22%", marginLeft: "auto" }}></span>
        </div>
      ))}
    </>
  );
}

export default function InstalledCard({
  data,
  onRefresh,
  onUnregisterProtocol,
  onClearCache,
  onRun,
  onProtocol,
  onShortcut,
  onOpen,
  onDelete,
}) {
  const [arm, setArm] = useState(null);
  const timer = useRef(null);

  useEffect(() => () => clearTimeout(timer.current), []);

  const requestDelete = (hash) => {
    if (arm === hash) {
      setArm(null);
      clearTimeout(timer.current);
      onDelete(hash);
      return;
    }
    setArm(hash);
    clearTimeout(timer.current);
    timer.current = setTimeout(() => setArm(null), 4000);
  };

  return (
    <section className="card" style={{ "--i": 3 }}>
      <div className="card-head">
        <h2>Trên máy</h2>
        <div className="card-actions">
          <button id="btn-refresh-installed" className="btn small icon" title="Quét lại" onClick={onRefresh}>
            <RefreshCw className="ic" />
          </button>
          <button id="btn-unregister-protocol" className="btn small" onClick={onUnregisterProtocol}>
            <Unlink className="ic" />
            Gỡ protocol
          </button>
          <button id="btn-clear-cache" className="btn small btn-danger-ghost" onClick={onClearCache}>
            <Broom className="ic" />
            Dọn cache
          </button>
        </div>
      </div>
      <div id="installed-list" className="list">
        {data.loading ? (
          <SkeletonRows />
        ) : data.items.length === 0 ? (
          <div className="empty">Không tìm thấy version nào trong %LOCALAPPDATA%\Roblox\Versions</div>
        ) : (
          data.items.map((v, i) => (
            <div className="row" key={v.hash} style={{ "--i": Math.min(i, 12) }}>
              <span className="hash-value">{v.hash}</span>
              <span className="badges">
                {v.hasPlayer ? <span className="badge ok">Player</span> : null}
                {v.hasStudio ? <span className="badge info">Studio</span> : null}
              </span>
              <span className="muted size">{v.sizeBytes ? fmtBytes(v.sizeBytes) : ""}</span>
              <span className="actions">
                <button
                  className="btn small icon"
                  title="Chạy Roblox Player"
                  onClick={() => onRun(v.hash)}
                  disabled={!v.hasPlayer}
                >
                  <Play className="ic" />
                </button>
                <button
                  className="btn small icon"
                  title="Đăng ký protocol roblox-player://"
                  onClick={() => onProtocol(v.hash)}
                  disabled={!v.hasPlayer}
                >
                  <Link className="ic" />
                </button>
                <button
                  className="btn small icon"
                  title="Tạo shortcut trên Desktop"
                  onClick={() => onShortcut(v.hash)}
                  disabled={!v.hasPlayer}
                >
                  <ExternalLink className="ic" />
                </button>
                <button
                  className="btn small icon"
                  title="Mở thư mục"
                  onClick={() => onOpen(v.path)}
                >
                  <Folder className="ic" />
                </button>
                <button
                  className={arm === v.hash ? "btn small danger" : "btn small icon"}
                  title="Xóa version"
                  onClick={() => requestDelete(v.hash)}
                >
                  {arm === v.hash ? "Xóa thật?" : <Trash2 className="ic" />}
                </button>
              </span>
            </div>
          ))
        )}
      </div>
    </section>
  );
}
