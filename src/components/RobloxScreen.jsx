import { useEffect, useState } from "react";
import { Check, ChevronDown, Download, History, RefreshCw, Square } from "lucide-react";
import { fmtEta, fmtSpeed } from "../lib/utils.js";
import { normalizeHash } from "../lib/hash.js";
import { executorIcon } from "../lib/executorIcons.js";
import ExecutorPreview from "./ExecutorPreview.jsx";
import InstalledCard from "./InstalledCard.jsx";

function ProgressBarBlock({ dlHash, dlBusy, dlDone, cancelReq, progress, onCancelDownload }) {
  const dlState = dlDone ? "done" : dlBusy ? "busy" : "idle";
  return (
    <div className="dl-panel" data-dl={dlState}>
      <div className="dl-inner">
        <div className="progress-label">
          <span
            id="dl-hash"
            className={dlState === "idle" ? "dl-hash idle" : "dl-hash"}
            title={dlDone ? "Hoàn tất" : dlHash || "Chưa có tác vụ tải nào"}
          >
            {dlDone ? `Hoàn tất — ${dlHash}` : dlHash || "Chưa có tác vụ tải nào"}
          </span>
          <span className="pct-wrap">
            {dlDone ? <Check className="done-check" /> : null}
            <span id="dl-overall-pct" className="pct pct-lg">
              {(Math.min(100, progress.overallPct ?? 0)).toFixed(1)}%
            </span>
          </span>
        </div>
        <div className="progress-bar">
          <div
            id="dl-bar-overall"
            className="progress-fill"
            style={{ "--pct": Math.min(100, progress.overallPct ?? 0) / 100 }}
          ></div>
        </div>
        <div className="progress-label sub">
          <span id="dl-pkg-name">{progress.pkgName || "Gói"}</span>
          <span id="dl-pkg-pct" className="pct">
            {Math.round(Math.min(100, progress.pkgPct ?? 0))}%
          </span>
        </div>
        <div className="progress-bar sm">
          <div
            id="dl-bar-pkg"
            className="progress-fill pkg"
            style={{ "--pct": Math.min(100, progress.pkgPct ?? 0) / 100 }}
          ></div>
        </div>
        <div className="dl-meta">
          <span id="dl-speed">{fmtSpeed(progress.speedBps)}</span>
          <span id="dl-eta">{fmtEta(progress.etaSecs)}</span>
          <button
            id="btn-cancel-download"
            className="btn small btn-danger-ghost"
            onClick={onCancelDownload}
            disabled={!dlBusy || cancelReq}
          >
            <Square className="ic" />
            Hủy tải
          </button>
        </div>
      </div>
    </div>
  );
}

const BINARY_TYPES = [
  { value: "player", label: "Windows Player" },
  { value: "studio", label: "Windows Studio" },
];

function PillIcon({ ex }) {
  const icon = executorIcon(ex.name);
  return icon ? <img className="pill-ic" src={icon} alt="" /> : null;
}

export default function RobloxScreen({
  live,
  liveBusy,
  onRefreshLive,
  customHash,
  onCustomHashChange,
  executors,
  executorsLoading,
  executorsLoaded,
  onLoadExecutors,
  dlHash,
  dlBusy,
  dlDone,
  cancelReq,
  progress,
  options,
  onOptionChange,
  onDownloadHash,
  onCancelDownload,
  installed,
  onRefreshInstalled,
  onUnregisterProtocol,
  onClearCache,
  onRunVersion,
  onProtocol,
  onShortcut,
  onOpen,
  onDeleteVersion,
}) {
  const [binaryType, setBinaryType] = useState("player");
  const [executorName, setExecutorName] = useState("");

  useEffect(() => {
    if (!executorsLoaded && !executorsLoading) onLoadExecutors();
  }, [executorsLoaded, executorsLoading, onLoadExecutors]);

  const hashCheck = normalizeHash(customHash);
  const executor = executors.find((e) => e.name === executorName) ?? null;
  const latestHash = binaryType === "studio" ? live?.studioLatest : live?.latest;
  const previousHash = binaryType === "studio" ? null : live?.previousWindows;

  return (
    <div className="screen">
      <section className="card hero" data-dl={dlDone ? "done" : dlBusy ? "busy" : "idle"} style={{ "--i": 0 }}>
        <div className="field">
          <label className="field-label" htmlFor="binary-type">Binary Type</label>
          <div className="select-wrap">
            <select
              id="binary-type"
              className="select"
              value={binaryType}
              onChange={(e) => setBinaryType(e.target.value)}
            >
              {BINARY_TYPES.map((b) => (
                <option value={b.value} key={b.value}>
                  {b.label}
                </option>
              ))}
            </select>
            <ChevronDown className="select-caret" />
          </div>
        </div>

        <div className="field">
          <label className="field-label" htmlFor="custom-hash">Version Hash (tùy chọn)</label>
          <input
            id="custom-hash"
            className="hash-input"
            type="text"
            placeholder="version-xxxxxxxxxxxxxxxx"
            spellCheck="false"
            autoComplete="off"
            value={customHash}
            onChange={(e) => onCustomHashChange(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter" && hashCheck.status === "ok" && !executor) {
                onDownloadHash(hashCheck.hash);
              }
            }}
          />
        </div>
        <span id="hash-preview" className={hashCheck.status === "invalid" ? "hint invalid" : "hint"}>
          {hashCheck.status === "invalid" ? "Hash không hợp lệ — cần 16 ký tự hex sau dấu gạch" : ""}
        </span>

        <div className="field">
          <label className="field-label" id="select-executor-label">Chọn Executor</label>
          <div className="executor-row">
            <div
              id="select-executor"
              className="executor-pills"
              role="group"
              aria-labelledby="select-executor-label"
            >
              {executorsLoading && executors.length === 0 ? (
                <span className="pills-hint muted">Đang nạp danh sách executor (WEAO + Umi)...</span>
              ) : null}
              {!executorsLoading && executorsLoaded && executors.length === 0 ? (
                <span className="pills-hint muted">Không nạp được executor — bấm nút làm mới.</span>
              ) : null}
              {executors.map((ex) => (
                <button
                  type="button"
                  key={ex.name}
                  className={executorName === ex.name ? "pill active" : "pill"}
                  onClick={() => setExecutorName(executorName === ex.name ? "" : ex.name)}
                  aria-pressed={executorName === ex.name}
                  title={ex.updateStatus ? "Đã cập nhật cho bản Roblox mới nhất" : "Chưa update — cần downgrade Roblox"}
                >
                  <PillIcon ex={ex} />
                  <span>{ex.name}</span>
                </button>
              ))}
            </div>
            <button
              className="btn small icon"
              title="Nạp lại danh sách executor (WEAO + Umi) — bỏ qua cache"
              onClick={() => onLoadExecutors(true)}
              disabled={executorsLoading}
            >
              <RefreshCw className={executorsLoading ? "ic spin" : "ic"} />
            </button>
          </div>
        </div>
        {executor ? <ExecutorPreview ex={executor} /> : null}

        {executor ? (
          <div className="btn-row dl-buttons">
            <button
              id="btn-download-executor"
              className="btn cta"
              onClick={() => onDownloadHash(executor.rbxVersion)}
              disabled={dlBusy}
            >
              <Download className="ic" />
              Tải bản {executor.name} Executor hỗ trợ
            </button>
          </div>
        ) : (
          <div className="btn-row dl-buttons">
            <button
              id="btn-download-latest"
              className="btn cta"
              onClick={() => {
                if (latestHash) onDownloadHash(latestHash);
                else if (!liveBusy) onRefreshLive();
              }}
              disabled={dlBusy || (liveBusy && !latestHash)}
              title={
                liveBusy && !latestHash
                  ? "Đang nạp dữ liệu phiên bản trực tiếp..."
                  : undefined
              }
            >
              <Download className="ic" />
              Tải bản mới nhất
            </button>
            <button
              id="btn-download-previous"
              className="btn"
              onClick={() => previousHash && onDownloadHash(previousHash)}
              disabled={dlBusy || !previousHash}
              title={
                previousHash
                  ? "Tải bản trước đó"
                  : binaryType === "studio"
                    ? "WEAO không có dữ liệu bản Studio trước đó"
                    : "Chưa có dữ liệu bản trước đó"
              }
            >
              <History className="ic" />
              Tải bản trước đó
            </button>
            <button
              id="btn-download-custom"
              className="btn"
              onClick={() => {
                if (hashCheck.status === "ok") onDownloadHash(hashCheck.hash);
              }}
              disabled={dlBusy || hashCheck.status !== "ok"}
              title="Tải theo Version Hash đã nhập"
            >
              <Download className="ic" />
              Tải bản chỉ định
            </button>
          </div>
        )}

        <div className="hero-options">
          <label className="check-label">
            <input
              type="checkbox"
              id="opt-register-protocol"
              checked={options.registerProtocol}
              onChange={(e) => onOptionChange("registerProtocol", e.target.checked)}
            />
            <span>Đăng ký protocol roblox-player:// sau khi tải xong</span>
          </label>
          <label className="check-label">
            <input
              type="checkbox"
              checked={options.createShortcut}
              onChange={(e) => onOptionChange("createShortcut", e.target.checked)}
            />
            <span>Tạo shortcut trên Desktop</span>
          </label>
          <label className="check-label">
            <input
              type="checkbox"
              checked={options.installWebview2}
              onChange={(e) => onOptionChange("installWebview2", e.target.checked)}
            />
            <span>Cài WebView2 Runtime im lặng</span>
          </label>
        </div>

        <ProgressBarBlock
          dlHash={dlHash}
          dlBusy={dlBusy}
          dlDone={dlDone}
          cancelReq={cancelReq}
          progress={progress}
          onCancelDownload={onCancelDownload}
        />
      </section>

      <InstalledCard
        data={installed}
        onRefresh={onRefreshInstalled}
        onUnregisterProtocol={onUnregisterProtocol}
        onClearCache={onClearCache}
        onRun={onRunVersion}
        onProtocol={onProtocol}
        onShortcut={onShortcut}
        onOpen={onOpen}
        onDelete={onDeleteVersion}
      />
    </div>
  );
}
