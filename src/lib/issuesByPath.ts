import type { Issue } from './validate';

export function issuesByPath(issues: Issue[]): Map<string, Issue[]> {
  const m = new Map<string, Issue[]>();
  for (const i of issues) {
    const cur = m.get(i.path);
    if (cur) cur.push(i);
    else m.set(i.path, [i]);
  }
  return m;
}
