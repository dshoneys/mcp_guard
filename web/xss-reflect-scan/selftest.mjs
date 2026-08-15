/**
 * Tiny Node self-check (no deps). Run: node selftest.mjs
 */
import {
  classifyReflection,
  htmlEscape,
  makeCanary,
  riskFlagFor,
  looksLikeHtml,
} from "./index.js";

const c = makeCanary("deadbeef");
const assert = (cond, msg) => {
  if (!cond) throw new Error(msg);
};

assert(
  classifyReflection(`<!DOCTYPE html><p>${c}</p>`, c) === "unescaped",
  "raw"
);
assert(
  classifyReflection(`<!DOCTYPE html><p>${htmlEscape(c)}</p>`, c) === "escaped",
  "escaped"
);
assert(
  classifyReflection("<!DOCTYPE html><html></html>", c) === "none",
  "none"
);
assert(looksLikeHtml("text/html", "<html>") === true, "html ct");
assert(looksLikeHtml("application/json", "{}") === false, "json ct");
assert(
  riskFlagFor({ outcome: "unescaped", path: "/", canary: c }) ===
    "xss_reflected_unescaped",
  "flag"
);
assert(riskFlagFor({ outcome: "escaped", path: "/", canary: c }) === null, "no flag");

console.log("xss-reflect-scan selftest ok");
