// Implements the asset-path rewrite rules from FORMAT.md §9.
//
// In the editor we don't always know the GitHub user/project pair — we
// only have a local folder. So we offer a "repo mode": when the user
// provides `user`, `project` (and optionally `ref`), we resolve bare-
// relative paths via jsDelivr just like the front-end would. Otherwise
// we return the path unchanged so the user sees what they wrote.

export interface RepoCoords {
  user: string;
  project: string;
  ref?: string; // defaults to HEAD
}

const JSDELIVR = 'https://cdn.jsdelivr.net/gh';

export function resolveAssetPath(src: string, repo?: RepoCoords | null): string {
  if (!src) return src;

  // Absolute & protocol-relative → unchanged
  if (/^https?:\/\//i.test(src) || src.startsWith('//')) return src;

  // /<otherUser>/<otherProject>[@<ref>]/assets/...
  // We detect the leading slash + first segment NOT being "assets".
  const crossRepo = src.match(
    /^\/(?!assets\/)([^/]+)\/([^/@]+)(?:@([^/]+))?\/(.+)$/
  );
  if (crossRepo) {
    const [, user, project, ref, rest] = crossRepo;
    return `${JSDELIVR}/${user}/${project}@${ref ?? 'HEAD'}/${rest}`;
  }

  if (!repo) return src;

  const ref = repo.ref ?? 'HEAD';

  // /assets/... → current repo on jsdelivr
  if (src.startsWith('/')) {
    return `${JSDELIVR}/${repo.user}/${repo.project}@${ref}${src}`;
  }

  // bare-relative → current repo on jsdelivr
  return `${JSDELIVR}/${repo.user}/${repo.project}@${ref}/${src}`;
}
