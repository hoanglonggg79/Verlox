import { Bot, CircleSlash, Coins, KeyRound, TriangleAlert } from "lucide-react";
import { executorIcon } from "../lib/executorIcons.js";

export default function ExecutorPreview({ ex }) {
  const icon = executorIcon(ex.name);
  return (
    <div className="ex-preview">
      <div className="ex-icon-wrap">
        {icon ? <img src={icon} alt={ex.name} /> : <Bot className="ic" />}
      </div>
      <div className="ex-info">
        <div className="ex-title-row">
          <span className="ex-name">{ex.name}</span>
          {ex.executorVersion ? <span className="ex-ver muted">v{ex.executorVersion}</span> : null}
          <span className="badges">
            {ex.updateStatus ? (
              <span className="badge ok">ĐÃ CẬP NHẬT</span>
            ) : (
              <span className="badge warn">CHƯA UPDATE</span>
            )}
          </span>
        </div>
        <div className="ex-rbx">
          Hỗ trợ Roblox: <code>{ex.rbxVersion}</code>
        </div>
        <div className="ex-chips">
          {ex.free ? (
            <span className="chip">
              <Coins className="ic" />
              Miễn phí
            </span>
          ) : ex.cost ? (
            <span className="chip">
              <Coins className="ic" />
              {ex.cost}
            </span>
          ) : null}
          {ex.keySystem == null ? null : ex.keySystem ? (
            <span className="chip">
              <KeyRound className="ic" />
              Key system
            </span>
          ) : (
            <span className="chip">
              <CircleSlash className="ic" />
              Không cần key
            </span>
          )}
        </div>
        {!ex.updateStatus ? (
          <div className="warn-box">
            <TriangleAlert className="ic warn-ic" />
            <div>
              <strong>Executor này chưa được cập nhật</strong>
              <p>
                Executor này chưa update cho bản Roblox mới nhất và cần Downgrade
                Roblox, tức là dùng bản cũ. Khi downgrade bạn có thể tiếp tục
                dùng trong khi đợi dev của executor đó cập nhật. Nhưng đổi lại
                sẽ tăng tỉ lệ ban do dùng bản cũ. Và nếu gặp Error Code 280
                khi join game, tức là phiên bản đó đã bị blocklisted từ Roblox.
              </p>
            </div>
          </div>
        ) : null}
      </div>
    </div>
  );
}
