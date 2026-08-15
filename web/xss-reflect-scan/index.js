/**
 * MCP Guard — reflected XSS canary probe (pure JS).
 * Same semantics as Rust `src/xss_reflect.rs` / REQ-SCAN-XSS-REFLECT.
 *
 * Browser note: reading another origin's response body requires CORS
 * (or same-origin). When the body cannot be read, outcome is `cors_blocked`.
 */

export const CANARY_MARKERS = "<>\"'";

export function makeCanary(token) {
  return `mgx${token}${CANARY_MARKERS}`;
}

export function htmlEscape(s) {
  return String(s)
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

/** @returns {"unescaped"|"escaped"|"none"} */
export function classifyReflection(body, canary) {
  if (body.includes(canary)) return "unescaped";
  if (body.includes(htmlEscape(canary))) return "escaped";
  return "none";
}

export function looksLikeHtml(contentType, body) {
  const ct = (contentType || "").toLowerCase();
  if (ct.includes("text/html") || ct.includes("application/xhtml")) return true;
  if (
    ct.includes("application/json") ||
    ct.includes("text/plain") ||
    ct.includes("application/javascript") ||
    ct.includes("text/css") ||
    ct.includes("image/")
  ) {
    return false;
  }
  const trim = String(body || "").trimStart();
  return (
    trim.startsWith("<!DOCTYPE") ||
    trim.startsWith("<!doctype") ||
    trim.startsWith("<html") ||
    trim.startsWith("<HTML")
  );
}

export function seedPaths(canary, max = 6) {
  const enc = encodeURIComponent(canary);
  const paths = [
    `/?q=${enc}`,
    `/search?q=${enc}`,
    `/error?msg=${enc}`,
    `/${enc}`,
    `/preview?url=${enc}`,
    `/sandbox-preview/${enc}/x`,
  ];
  return paths.slice(0, Math.max(1, max));
}

export function riskFlagFor(probe) {
  return probe && probe.outcome === "unescaped"
    ? "xss_reflected_unescaped"
    : null;
}

/**
 * Probe one HTTP origin (e.g. http://127.0.0.1:3088).
 * @param {object} opts
 * @param {string} opts.baseUrl - origin without trailing slash
 * @param {number} [opts.maxProbes=6]
 * @param {typeof fetch} [opts.fetchImpl]
 * @param {string} [opts.token] - canary token; default from baseUrl hash
 */
export async function probeOrigin(opts) {
  const baseUrl = String(opts.baseUrl).replace(/\/$/, "");
  const maxProbes = opts.maxProbes ?? 6;
  const fetchImpl = opts.fetchImpl || globalThis.fetch;
  if (!fetchImpl) throw new Error("fetch unavailable");

  const token =
    opts.token ||
    Math.abs(
      [...baseUrl].reduce((a, c) => ((a << 5) - a + c.charCodeAt(0)) | 0, 0)
    )
      .toString(16)
      .padStart(8, "0")
      .slice(0, 8);
  const canary = makeCanary(token);
  const paths = seedPaths(canary, maxProbes);

  let sawHtml = false;
  let bestEscaped = null;
  let corsBlocked = false;

  for (const path of paths) {
    const url = baseUrl + path;
    let res;
    try {
      res = await fetchImpl(url, {
        method: "GET",
        mode: "cors",
        credentials: "omit",
        headers: { Accept: "text/html,*/*" },
      });
    } catch {
      corsBlocked = true;
      continue;
    }
    let body = "";
    try {
      body = await res.text();
    } catch {
      corsBlocked = true;
      continue;
    }
    const ct = res.headers.get("content-type");
    if (!looksLikeHtml(ct, body)) continue;
    sawHtml = true;
    const kind = classifyReflection(body, canary);
    if (kind === "unescaped") {
      return { outcome: "unescaped", path, canary, baseUrl };
    }
    if (kind === "escaped" && !bestEscaped) {
      bestEscaped = { outcome: "escaped", path, canary, baseUrl };
    }
  }

  if (bestEscaped) return bestEscaped;
  if (sawHtml) return { outcome: "html_no_reflect", path: "/", canary, baseUrl };
  if (corsBlocked) return { outcome: "cors_blocked", path: "/", canary, baseUrl };
  return { outcome: "none", path: "/", canary, baseUrl };
}

/**
 * Scan many ports on a host (browser or Node).
 * @param {object} opts
 * @param {string} [opts.host="127.0.0.1"]
 * @param {number[]} opts.ports
 * @param {number} [opts.maxProbes=6]
 * @param {typeof fetch} [opts.fetchImpl]
 */
export async function scanPorts(opts) {
  const host = opts.host || "127.0.0.1";
  const ports = opts.ports || [];
  const findings = [];
  for (const port of ports) {
    const baseUrl = `http://${host}:${port}`;
    const xss = await probeOrigin({
      baseUrl,
      maxProbes: opts.maxProbes,
      fetchImpl: opts.fetchImpl,
    });
    const flag = riskFlagFor(xss);
    findings.push({
      port,
      xss,
      risk_flags: flag ? [flag] : [],
    });
  }
  return {
    host,
    scanned_at: new Date().toISOString(),
    findings,
  };
}
