import json
import os

log_path = r"C:\Users\zzy\.gemini\antigravity-ide\brain\e949017e-548e-4db0-8d0d-00a057619e07\.system_generated\logs\transcript.jsonl"
if not os.path.exists(log_path):
    print("Log path does not exist:", log_path)
    exit(1)

with open(log_path, 'r', encoding='utf-8') as f:
    for line in f:
        try:
            data = json.loads(line)
            if data.get("step_index") == 311:
                tc = data["tool_calls"][0]
                code = tc["args"]["CodeContent"]
                if isinstance(code, str):
                    if code.startswith('"') and code.endswith('"'):
                        code = json.loads(code)
                    out_path = r"c:\Users\zzy\Desktop\pyautogui\whac_a_mole.py"
                    with open(out_path, 'w', encoding='utf-8') as outf:
                        outf.write(code)
                    print(f"Restored successfully to {out_path}!")
                else:
                    print("Code was not a string:", type(code))
                break
        except Exception as e:
            pass
