import { openUrl } from "@tauri-apps/plugin-opener";
import { Braces, GitBranch, User, Video, Zap } from "lucide-react";
import { hasTauri } from "../lib/tauri.js";
import { DEV_DISCORD_URL, DEV_YOUTUBE_URL, REPO_URL, WEAO_URL } from "../lib/links.js";

export default function HomeScreen({ onLog }) {
  const open = async (url, note) => {
    if (!url) {
      onLog(note, "warn");
      return;
    }
    try {
      await openUrl(url);
    } catch (err) {
      onLog(String(err), "error");
    }
  };

  return (
    <div className="home">
      <section className="card hero home-hero" style={{ "--i": 0 }}>
        <div className="home-logo">
          VERLOX<span className="brand-cursor">_</span>
        </div>
        <p className="home-tagline">
          Launcher & trình quản lý phiên bản Roblox trên Windows: tải đúng bản
          Roblox mà executor của bạn hỗ trợ, quản lý các bản đã cài, ghim
          protocol roblox-player:// và đồng bộ lịch sử tải.
        </p>
        <div className="home-pow">
          <Zap className="ic" />
          <span>API Powered by </span>
          <button className="link" onClick={() => open(WEAO_URL)}>
            WEAO (weao.xyz)
          </button>
        </div>
      </section>

      <div className="cards">
        <section className="card home-card" style={{ "--i": 1 }}>
          <div className="home-card-icon">
            <User className="ic" />
          </div>
          <h2>Người phát triển</h2>
          <p className="muted">Liên hệ / kết nối với dev của Verlox trên Discord.</p>
          <button className="btn small" onClick={() => open(DEV_DISCORD_URL)}>
            <User className="ic" />
            Dev trên Discord
          </button>
        </section>

        <section className="card home-card" style={{ "--i": 2 }}>
          <div className="home-card-icon">
            <Video className="ic" />
          </div>
          <h2>Kênh YouTube</h2>
          <p className="muted">Video hướng dẫn, cập nhật và demo của dev.</p>
          <button className="btn small" onClick={() => open(DEV_YOUTUBE_URL)}>
            <Video className="ic" />
            @LongHoang-2105
          </button>
        </section>

        <section className="card home-card" style={{ "--i": 3 }}>
          <div className="home-card-icon">
            <GitBranch className="ic" />
          </div>
          <h2>Repository</h2>
          <p className="muted">Mã nguồn Verlox — Tauri 2, Rust backend, MIT.</p>
          <button
            className="btn small"
            onClick={() => open(REPO_URL, "Chưa cấu hình REPO_URL — điền URL repo vào src/lib/links.js")}
          >
            <GitBranch className="ic" />
            Repo Verlox
          </button>
        </section>

        <section className="card home-card" style={{ "--i": 4 }}>
          <div className="home-card-icon">
            <Braces className="ic" />
          </div>
          <h2>Thông tin bản dựng</h2>
          <div className="home-meta">
            <span>Verlox v1.2.0</span>
            <span>Rust + Tauri 2 · React 19 + Vite</span>
            <span>License MIT</span>
          </div>
        </section>
      </div>
    </div>
  );
}
