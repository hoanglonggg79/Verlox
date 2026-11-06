import { useCallback, useEffect, useRef, useState } from "react";
import { hasTauri, invoke, listen } from "./lib/tauri.js";
import Sidebar from "./components/Sidebar.jsx";
import HomeScreen from "./components/HomeScreen.jsx";
import RobloxScreen from "./components/RobloxScreen.jsx";
import LogsScreen from "./components/LogsScreen.jsx";

const LOG_LEVEL_MAP = {
  muted: "muted",
  warning: "warn",
  success: "ok",
  error: "error",
  info: "info",
};
const MAX_LOG = 300;
const EXISTS_PREFIX = "EXISTS::";
const EMPTY_PROGRESS = {
  overallPct: 0,
  pkgName: "",
  pkgPct: 0,
  speedBps: 0,
  etaSecs: 0,
};

let logSeq = 0;

export default function App() {
  const [tab, setTab] = useState("home");
  const [logs, setLogs] = useState([]);
  const [live, setLive] = useState(null);
  const [liveBusy, setLiveBusy] = useState(false);
  const [installed, setInstalled] = useState({ loading: false, items: [] });
  const [history, setHistory] = useState(null);
  const [deployVersions, setDeployVersions] = useState([]);
  const [executors, setExecutors] = useState([]);
  const [executorsLoading, setExecutorsLoading] = useState(false);
  const [executorsLoaded, setExecutorsLoaded] = useState(false);
  const [customHash, setCustomHash] = useState("");
  const [dlHash, setDlHash] = useState("");
  const [dlBusy, setDlBusy] = useState(false);
  const [dlDone, setDlDone] = useState(false);
  const [cancelReq, setCancelReq] = useState(false);
  const [progress, setProgress] = useState(EMPTY_PROGRESS);
  const [options, setOptions] = useState({
    registerProtocol: false,
    installWebview2: false,
    createShortcut: false,
  });

  const doneTimer = useRef(null);
  const dlHashRef = useRef("");

  const pushLog = useCallback((message, kind = "info") => {
    const entry = { id: ++logSeq, t: Date.now(), kind, message };
    setLogs((prev) => {
      const next = [...prev, entry];
      return next.length > MAX_LOG ? next.slice(next.length - MAX_LOG) : next;
    });
  }, []);

  const refreshLive = useCallback(async () => {
    if (!hasTauri) return;
    setLiveBusy(true);
    try {
      const data = await invoke("get_live_versions");
      setLive(data);
      pushLog("Đã nạp thông tin phiên bản trực tiếp", "ok");
    } catch (err) {
      pushLog(String(err), "error");
    } finally {
      setLiveBusy(false);
    }
  }, [pushLog]);

  const refreshInstalled = useCallback(
    async (withSizes = true) => {
      if (!hasTauri) return;
      setInstalled((s) => ({ ...s, loading: true }));
      try {
        const items = await invoke("list_installed_versions", { withSizes });
        setInstalled({ loading: false, items });
        pushLog(`Tìm thấy ${items.length} version trên máy`, "ok");
      } catch (err) {
        setInstalled((s) => ({ ...s, loading: false }));
        pushLog(String(err), "error");
      }
    },
    [pushLog]
  );

  const refreshHistory = useCallback(async () => {
    if (!hasTauri) return;
    try {
      const entries = await invoke("get_download_history");
      setHistory(entries);
    } catch (err) {
      pushLog(String(err), "error");
    }
  }, [pushLog]);

  const loadExecutors = useCallback(async () => {
    if (!hasTauri || executorsLoading) return;
    setExecutorsLoading(true);
    try {
      const [weaoRes, umiRes] = await Promise.allSettled([
        invoke("list_executors"),
        invoke("get_umi_executor"),
      ]);
      if (weaoRes.status === "rejected") pushLog(String(weaoRes.reason), "error");
      if (umiRes.status === "rejected") pushLog(String(umiRes.reason), "error");
      const list = weaoRes.status === "fulfilled" ? [...weaoRes.value] : [];
      if (
        umiRes.status === "fulfilled" &&
        !list.some((e) => e.name.toLowerCase() === umiRes.value.name.toLowerCase())
      ) {
        list.push(umiRes.value);
      }
      setExecutors(list);
      setExecutorsLoaded(true);
      if (list.length > 0) {
        const sources = [];
        if (weaoRes.status === "fulfilled") sources.push("WEAO");
        if (umiRes.status === "fulfilled") sources.push("Umi");
        pushLog(`Nạp ${list.length} executor Windows (${sources.join(" + ")})`, "ok");
      }
    } finally {
      setExecutorsLoading(false);
    }
  }, [executorsLoading, pushLog]);

  const openFolder = useCallback(
    async (p) => {
      try {
        await invoke("open_folder", { path: p });
      } catch (err) {
        pushLog(String(err), "error");
      }
    },
    [pushLog]
  );

  const resetDlVisuals = useCallback(() => {
    setDlHash("");
    setProgress(EMPTY_PROGRESS);
  }, []);

  const markDownloadDone = useCallback(
    (dir) => {
      const hash = dlHashRef.current;
      setDlBusy(false);
      setCancelReq(false);
      clearTimeout(doneTimer.current);
      if (dir) {
        setDlDone(true);
        setProgress((p) => ({
          ...p,
          overallPct: 100,
          pkgPct: 100,
          speedBps: 0,
          etaSecs: 0,
        }));
        doneTimer.current = setTimeout(() => {
          setDlDone(false);
          resetDlVisuals();
        }, 2600);
        if (
          window.confirm(
            `Roblox đã sẵn sàng! (${hash})\n\nThư mục:\n${dir}\n\nMở thư mục cài đặt?`
          )
        ) {
          openFolder(dir);
        }
      } else {
        setDlDone(false);
        resetDlVisuals();
      }
    },
    [openFolder, resetDlVisuals]
  );

  const startDownload = useCallback(
    async (hash, overwrite = false) => {
      const opts = { ...options, overwrite };
      try {
        await invoke("start_download", { hash, installDir: null, options: opts });
        clearTimeout(doneTimer.current);
        dlHashRef.current = hash;
        setDlHash(hash);
        setDlBusy(true);
        setDlDone(false);
        setCancelReq(false);
        setProgress(EMPTY_PROGRESS);
        pushLog(`Bắt đầu tải ${hash}`, "ok");
      } catch (err) {
        const msg = String(err);
        if (msg.startsWith(EXISTS_PREFIX)) {
          const dir = msg.slice(EXISTS_PREFIX.length);
          if (
            window.confirm(
              `Phiên bản đã được cài tại:\n${dir}\n\nCài đè (ghi đè lên thư mục cũ)?`
            )
          ) {
            startDownload(hash, true);
          } else {
            pushLog("Đã hủy — thư mục cài đè đã tồn tại", "muted");
          }
        } else {
          pushLog(msg, "error");
        }
      }
    },
    [options, pushLog]
  );

  const cancelDownload = useCallback(async () => {
    try {
      await invoke("cancel_download");
      setCancelReq(true);
      pushLog("Đang hủy... — .part được giữ lại để resume lần sau", "warn");
    } catch (err) {
      pushLog(String(err), "error");
    }
  }, [pushLog]);

  const loadDeployHistory = useCallback(async () => {
    try {
      const versions = await invoke("get_history_versions");
      setDeployVersions(versions);
      pushLog(`Nạp ${versions.length} version từ DeployHistory.txt`, "ok");
    } catch (err) {
      pushLog(String(err), "error");
    }
  }, [pushLog]);

  const runVersion = useCallback(
    async (h) => {
      try {
        await invoke("launch_version", { hash: h });
        pushLog(`Đã khởi chạy ${h}`, "ok");
      } catch (err) {
        pushLog(String(err), "error");
      }
    },
    [pushLog]
  );

  const registerProtocol = useCallback(
    async (h) => {
      try {
        await invoke("register_protocol", { hash: h });
        pushLog(`Đã đăng ký protocol roblox-player:// cho ${h}`, "ok");
      } catch (err) {
        pushLog(String(err), "error");
      }
    },
    [pushLog]
  );

  const registerShortcut = useCallback(
    async (h) => {
      try {
        await invoke("create_shortcut", { hash: h });
        pushLog(`Đã tạo shortcut trên Desktop cho ${h}`, "ok");
      } catch (err) {
        pushLog(String(err), "error");
      }
    },
    [pushLog]
  );

  const unregisterProtocol = useCallback(async () => {
    try {
      await invoke("unregister_protocol");
      pushLog("Đã gỡ đăng ký protocol roblox-player://", "ok");
    } catch (err) {
      pushLog(String(err), "error");
    }
  }, [pushLog]);

  const deleteVersion = useCallback(
    async (h) => {
      try {
        await invoke("delete_version", { hash: h });
        pushLog(`Đã xóa ${h}`, "ok");
        refreshInstalled(true);
      } catch (err) {
        pushLog(String(err), "error");
      }
    },
    [pushLog, refreshInstalled]
  );

  const clearCache = useCallback(async () => {
    try {
      const removed = await invoke("clear_cache");
      pushLog(`Đã dọn cache: giải phóng ${fmtRemoved(removed)}`, "ok");
    } catch (err) {
      pushLog(String(err), "error");
    }
  }, [pushLog]);

  const importLegacyHistory = useCallback(async () => {
    const p = window.prompt("Đường dẫn đến download_history.json cũ:", "");
    if (!p) return;
    try {
      const saved = await invoke("import_legacy_history", { filePath: p });
      pushLog(`Đã import lịch sử (${saved} bản ghi)`, "ok");
      refreshHistory();
    } catch (err) {
      pushLog(String(err), "error");
    }
  }, [pushLog, refreshHistory]);

  const focusInput = useCallback(() => {
    requestAnimationFrame(() => {
      document.getElementById("custom-hash")?.focus();
    });
  }, []);

  const pickDeployHash = useCallback(
    (hash) => {
      setCustomHash(hash);
      focusInput();
    },
    [focusInput]
  );

  const setCustomFromHistory = useCallback(
    (hash) => {
      setCustomHash(hash);
      focusInput();
    },
    [focusInput]
  );

  useEffect(() => {
    if (!hasTauri) {
      pushLog("Tauri API không có — hãy chạy qua `npm run dev`", "error");
      return;
    }
    let alive = true;
    const offs = [];
    (async () => {
      offs.push(
        await listen("download://log", (e) => {
          const { message, level } = e.payload;
          pushLog(message, LOG_LEVEL_MAP[level] ?? "info");
        })
      );
      offs.push(
        await listen("download://progress", (e) => {
          setProgress((prev) => ({ ...prev, ...e.payload }));
        })
      );
      offs.push(
        await listen("download://done", (e) => {
          const dir = e.payload?.installDir ?? null;
          markDownloadDone(dir);
          if (dir) {
            refreshHistory();
            refreshInstalled(true);
          }
        })
      );
      offs.push(
        await listen("download://error", (e) => {
          const { title, message } = e.payload;
          setDlBusy(false);
          setCancelReq(false);
          setDlDone(false);
          clearTimeout(doneTimer.current);
          pushLog(`Lỗi — ${title}: ${message}`, "error");
        })
      );
      if (!alive) offs.forEach((f) => f());
    })();
    refreshLive();
    refreshInstalled(true);
    refreshHistory();
    return () => {
      alive = false;
      offs.forEach((f) => f());
      clearTimeout(doneTimer.current);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <div className="app-shell">
      <Sidebar active={tab} onSelect={setTab} />
      <div className="content">
        {tab === "home" ? <HomeScreen onLog={pushLog} /> : null}
        {tab === "roblox" ? (
          <RobloxScreen
            live={live}
            liveBusy={liveBusy}
            onRefreshLive={refreshLive}
            customHash={customHash}
            onCustomHashChange={setCustomHash}
            executors={executors}
            executorsLoading={executorsLoading}
            executorsLoaded={executorsLoaded}
            onLoadExecutors={loadExecutors}
            dlHash={dlHash}
            dlBusy={dlBusy}
            dlDone={dlDone}
            cancelReq={cancelReq}
            progress={progress}
            options={options}
            onOptionChange={(key, value) => setOptions((s) => ({ ...s, [key]: value }))}
            onDownloadHash={startDownload}
            onCancelDownload={cancelDownload}
            deployVersions={deployVersions}
            onLoadDeploy={loadDeployHistory}
            onPickDeploy={pickDeployHash}
            installed={installed}
            onRefreshInstalled={() => refreshInstalled(true)}
            onUnregisterProtocol={unregisterProtocol}
            onClearCache={clearCache}
            onRunVersion={runVersion}
            onProtocol={registerProtocol}
            onShortcut={registerShortcut}
            onOpen={openFolder}
            onDeleteVersion={deleteVersion}
            history={history}
            onImportHistory={importLegacyHistory}
            onFillHash={setCustomFromHistory}
          />
        ) : null}
        {tab === "logs" ? <LogsScreen logs={logs} /> : null}
      </div>
    </div>
  );
}

function fmtRemoved(n) {
  if (!n) return "0 B";
  if (n < 1048576) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / 1048576).toFixed(2)} MB`;
}
