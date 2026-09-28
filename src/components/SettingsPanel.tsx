import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { beginDialog, endDialog, openLogDir, resetUsageRoots, scaleOptions, setScale, setUsageRoots, usageRoots } from "../api";
import type { Tool, ToolRoots } from "../types";

const labels: Record<Tool, string> = { "claude-code": "Claude Code", codex: "Codex", "gemini-cli": "Gemini CLI" };

export function SettingsPanel({ onBack }: { onBack: () => void }) {
  const [roots, setRoots] = useState<ToolRoots[]>([]);
  const [scales, setScales] = useState<number[]>([1]);
  const [scale, setCurrentScale] = useState(1);
  const [error, setError] = useState("");
  const load = async () => { try { const [nextRoots, nextScales] = await Promise.all([usageRoots(), scaleOptions()]); setRoots(nextRoots); setScales(nextScales); } catch (reason) { setError(String(reason)); } };
  useEffect(() => { void load(); }, []);
  const update = async (tool: Tool, paths: string[]) => { try { setRoots(await setUsageRoots(tool, paths)); } catch (reason) { setError(String(reason)); } };
  const addPath = async (root: ToolRoots) => {
    await beginDialog();
    try {
      const path = await open({ directory: true, multiple: false });
      if (typeof path === "string") void update(root.tool, [...root.custom.map(item => item.path), path]);
    } finally {
      await endDialog();
    }
  };
  const reset = async () => { if (window.confirm("수집 경로를 모두 초기화할까요?")) try { setRoots(await resetUsageRoots()); } catch (reason) { setError(String(reason)); } };
  const chooseScale = async (value: number) => { try { await setScale(value); setCurrentScale(value); } catch (reason) { setError(String(reason)); } };
  return <main className="panel settings">
    <header><button className="back" onClick={onBack} aria-label="뒤로">←</button><strong>설정</strong></header>
    <label className="check"><input type="checkbox" checked={false} disabled readOnly /> 로그인 시 자동 시작</label>
    <label className="scale">창 배율 <select value={scale} onChange={event => void chooseScale(Number(event.target.value))}>{scales.map(value => <option key={value} value={value}>{value * 100}%</option>)}</select></label>
    <section><h2>수집 경로</h2>{roots.map(root => <div className="root-group" key={root.tool}><h3>{labels[root.tool]}</h3>{root.defaults.map(item => <Path key={item.path} item={item} />)}{root.custom.map(item => <Path key={item.path} item={item} remove={() => void update(root.tool, root.custom.filter(path => path.path !== item.path).map(path => path.path))} />)}<button className="add" onClick={() => void addPath(root)}>+ 경로 추가</button></div>)}<button className="reset" onClick={() => void reset()}>수집 경로 전체 초기화</button></section>
    <footer><button onClick={() => void openLogDir()}>로그 폴더 열기</button></footer>{error && <p className="message">{error}</p>}
  </main>;
}

function Path({ item, remove }: { item: ToolRoots["defaults"][number]; remove?: () => void }) {
  return <div className={`path ${item.exists ? "" : "missing"}`}><span>{item.path}</span><small>{item.exists ? `${item.files}개` : "(없음)"}</small>{remove && <button onClick={remove} aria-label={`${item.path} 삭제`}>×</button>}</div>;
}
