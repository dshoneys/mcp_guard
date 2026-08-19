//! Locale catalogs under `ui/i18n/{locale}.toml`. Default: zh-CN.

use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::json;
use std::path::PathBuf;

pub const DEFAULT_LOCALE: &str = "zh-CN";

#[derive(Debug, Clone, Deserialize)]
pub struct Catalog {
    #[serde(default)]
    pub status: StatusStrings,
    #[serde(default)]
    pub tray: TrayStrings,
    #[serde(default)]
    pub dashboard: DashboardStrings,
    #[serde(default)]
    pub vault: VaultStrings,
    #[serde(default)]
    pub toast: ToastStrings,
    /// Machine risk flag → human-readable description.
    #[serde(default)]
    pub flags: FlagStrings,
    #[serde(default)]
    pub plugins: PluginStrings,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StatusStrings {
    #[serde(default = "zh_idle")]
    pub idle: String,
    #[serde(default = "zh_exposure")]
    pub exposure: String,
    #[serde(default = "zh_activity")]
    pub activity: String,
    #[serde(default = "zh_scanning")]
    pub scanning: String,
    #[serde(default = "zh_muted_suffix")]
    pub muted_suffix: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TrayStrings {
    #[serde(default = "zh_open_dashboard")]
    pub open_dashboard: String,
    #[serde(default = "zh_open_audit")]
    pub open_audit: String,
    #[serde(default = "zh_scan_now")]
    pub scan_now: String,
    #[serde(default = "zh_mute")]
    pub mute: String,
    #[serde(default = "zh_quit")]
    pub quit: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DashboardStrings {
    #[serde(default = "zh_tagline")]
    pub tagline: String,
    #[serde(default = "zh_last_scan")]
    pub last_scan: String,
    #[serde(default = "zh_metric_open")]
    pub metric_open: String,
    #[serde(default = "zh_metric_exposure")]
    pub metric_exposure: String,
    #[serde(default = "zh_metric_activity")]
    pub metric_activity: String,
    #[serde(default = "zh_dash_scan")]
    pub scan_now: String,
    #[serde(default = "zh_dash_audit")]
    pub open_audit: String,
    #[serde(default = "zh_open_audit_tip")]
    pub open_audit_tip: String,
    #[serde(default = "zh_dash_mute")]
    pub mute: String,
    #[serde(default = "zh_mute_tip")]
    pub mute_tip: String,
    #[serde(default = "zh_footer")]
    pub footer: String,
    #[serde(default = "zh_preview_scan")]
    pub preview_scan_warn: String,
    #[serde(default = "zh_preview_audit")]
    pub preview_audit: String,
    #[serde(default = "zh_preview_mute")]
    pub preview_mute: String,
    #[serde(default = "zh_scan_panel_title")]
    pub scan_panel_title: String,
    #[serde(default = "zh_scan_panel_waiting")]
    pub scan_panel_waiting: String,
    #[serde(default = "zh_scan_panel_scanning")]
    pub scan_panel_scanning: String,
    #[serde(default = "zh_scan_panel_ok")]
    pub scan_panel_ok: String,
    #[serde(default = "zh_scan_panel_warn")]
    pub scan_panel_warn: String,
    #[serde(default = "zh_scan_panel_danger")]
    pub scan_panel_danger: String,
    #[serde(default = "zh_scan_panel_error")]
    pub scan_panel_error: String,
    #[serde(default = "zh_open_plugins")]
    pub open_plugins: String,
    #[serde(default = "zh_open_vault")]
    pub open_vault: String,
    #[serde(default = "zh_back_home")]
    pub back_home: String,
    #[serde(default = "zh_back_plugins")]
    pub back_plugins: String,
    #[serde(default = "zh_risks_title")]
    pub risks_title: String,
    #[serde(default = "zh_risk_empty")]
    pub risk_empty: String,
    #[serde(default = "zh_risk_exposure")]
    pub risk_exposure: String,
    #[serde(default = "zh_risk_activity")]
    pub risk_activity: String,
    #[serde(default = "zh_risk_flags")]
    pub risk_flags: String,
    #[serde(default = "zh_risk_app_unknown")]
    pub risk_app_unknown: String,
    #[serde(default = "zh_risk_kind_exposure")]
    pub risk_kind_exposure: String,
    #[serde(default = "zh_risk_kind_warn")]
    pub risk_kind_warn: String,
    #[serde(default = "zh_risk_kind_mcp")]
    pub risk_kind_mcp: String,
    #[serde(default = "zh_risk_kind_activity")]
    pub risk_kind_activity: String,
    #[serde(default = "zh_risk_mcp_line")]
    pub risk_mcp_line: String,
    #[serde(default = "zh_risk_allow")]
    pub risk_allow: String,
    #[serde(default = "zh_preview_allow")]
    pub preview_allow: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FlagStrings {
    #[serde(default = "zh_flag_cors_star")]
    pub cors_star: String,
    #[serde(default = "zh_flag_open_http_no_cors")]
    pub open_http_no_cors: String,
    #[serde(default = "zh_flag_mcp_tools")]
    pub mcp_tools_exposed: String,
    #[serde(default = "zh_flag_mcp_jsonrpc")]
    pub mcp_jsonrpc_surface: String,
    #[serde(default = "zh_flag_no_www")]
    pub no_www_authenticate_hint: String,
    #[serde(default = "zh_flag_workbuddy")]
    pub known_workbuddy_ardot_port: String,
    #[serde(default = "zh_flag_tcp")]
    pub tcp_open_non_http_or_timeout: String,
    #[serde(default = "zh_flag_unknown_client")]
    pub unknown_client: String,
    #[serde(default = "zh_flag_xss_reflect")]
    pub xss_reflected_unescaped: String,
    /// Fallback when flag id has no translation: `{flag}`
    #[serde(default = "zh_flag_unknown")]
    pub unknown: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PluginStrings {
    #[serde(default = "zh_plugins_title")]
    pub title: String,
    #[serde(default = "zh_plugins_hint")]
    pub hint: String,
    #[serde(default = "zh_plugin_vault_title")]
    pub vault_title: String,
    #[serde(default = "zh_plugin_vault_desc")]
    pub vault_desc: String,
    #[serde(default = "zh_plugin_coming_title")]
    pub coming_title: String,
    #[serde(default = "zh_plugin_coming_desc")]
    pub coming_desc: String,
    #[serde(default = "zh_plugin_coming_badge")]
    pub coming_badge: String,
    #[serde(default = "zh_plugin_open")]
    pub open: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VaultStrings {
    #[serde(default = "zh_vault_title")]
    pub title: String,
    #[serde(default = "zh_vault_hint")]
    pub hint: String,
    #[serde(default = "zh_vault_name")]
    pub name_placeholder: String,
    #[serde(default = "zh_vault_value")]
    pub value_placeholder: String,
    #[serde(default = "zh_vault_save")]
    pub save: String,
    #[serde(default = "zh_vault_delete")]
    pub delete: String,
    #[serde(default = "zh_vault_empty")]
    pub empty: String,
    #[serde(default = "zh_vault_confirm")]
    pub confirm_delete: String,
    #[serde(default = "zh_vault_preview_saved")]
    pub preview_saved: String,
    #[serde(default = "zh_vault_preview_deleted")]
    pub preview_deleted: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ToastStrings {
    #[serde(default = "zh_toast_act_t")]
    pub scan_activity_title: String,
    #[serde(default = "zh_toast_act_b")]
    pub scan_activity_body: String,
    #[serde(default = "zh_toast_exp_t")]
    pub scan_exposure_title: String,
    #[serde(default = "zh_toast_exp_b")]
    pub scan_exposure_body: String,
    #[serde(default = "zh_toast_ok_t")]
    pub scan_ok_title: String,
    #[serde(default = "zh_toast_ok_b")]
    pub scan_ok_body: String,
    #[serde(default = "zh_toast_esc_act")]
    pub escalation_activity_body: String,
    #[serde(default = "zh_toast_esc_exp")]
    pub escalation_exposure_body: String,
    #[serde(default = "zh_toast_mute_t")]
    pub mute_title: String,
    #[serde(default = "zh_toast_mute_b")]
    pub mute_body: String,
    #[serde(default = "zh_toast_audit_fail")]
    pub audit_fail_title: String,
    #[serde(default = "zh_toast_scan_fail")]
    pub scan_fail_title: String,
    #[serde(default = "zh_toast_dash_fail")]
    pub dashboard_fail_title: String,
    #[serde(default = "zh_toast_vault_t")]
    pub vault_title: String,
    #[serde(default = "zh_toast_vault_saved")]
    pub vault_saved: String,
    #[serde(default = "zh_toast_vault_del")]
    pub vault_deleted: String,
    #[serde(default = "zh_toast_vault_miss")]
    pub vault_missing: String,
    #[serde(default = "zh_toast_vault_sf")]
    pub vault_save_fail_title: String,
    #[serde(default = "zh_toast_vault_df")]
    pub vault_delete_fail_title: String,
    #[serde(default = "zh_toast_allow_t")]
    pub allow_title: String,
    #[serde(default = "zh_toast_allow_saved")]
    pub allow_saved: String,
    #[serde(default = "zh_toast_allow_fail")]
    pub allow_fail_title: String,
}

fn zh_idle() -> String {
    "MCP Guard — 正常".into()
}
fn zh_exposure() -> String {
    "暴露告警".into()
}
fn zh_activity() -> String {
    "可疑活动".into()
}
fn zh_scanning() -> String {
    "扫描中…".into()
}
fn zh_muted_suffix() -> String {
    "（弹窗已暂停）".into()
}
fn zh_open_dashboard() -> String {
    "打开主面板".into()
}
fn zh_open_audit() -> String {
    "打开扫描日志".into()
}
fn zh_scan_now() -> String {
    "立即扫描".into()
}
fn zh_mute() -> String {
    "暂停弹窗（1 小时）".into()
}
fn zh_quit() -> String {
    "退出".into()
}
fn zh_tagline() -> String {
    "本地 MCP 表面哨兵".into()
}
fn zh_last_scan() -> String {
    "上次扫描".into()
}
fn zh_metric_open() -> String {
    "开放服务".into()
}
fn zh_metric_exposure() -> String {
    "暴露".into()
}
fn zh_metric_activity() -> String {
    "活动告警".into()
}
fn zh_dash_scan() -> String {
    "立即扫描".into()
}
fn zh_dash_audit() -> String {
    "打开日志".into()
}
fn zh_open_audit_tip() -> String {
    "在资源管理器中定位扫描日志文件（仍会继续记录）".into()
}
fn zh_dash_mute() -> String {
    "暂停弹窗 1 小时".into()
}
fn zh_mute_tip() -> String {
    "暂时不弹桌面通知；后台扫描与日志照常".into()
}
fn zh_footer() -> String {
    "托盘常驻".into()
}
fn zh_preview_scan() -> String {
    "预览：检测到暴露（演示数据）。".into()
}
fn zh_preview_audit() -> String {
    "预览：将在资源管理器中打开扫描日志。".into()
}
fn zh_preview_mute() -> String {
    "预览：桌面弹窗已暂停 1 小时（扫描仍继续）。".into()
}
fn zh_scan_panel_title() -> String {
    "扫描结果".into()
}
fn zh_scan_panel_waiting() -> String {
    "尚未扫描。点「立即扫描」查看本地 MCP 表面。".into()
}
fn zh_scan_panel_scanning() -> String {
    "正在扫描本地端口与连接…".into()
}
fn zh_scan_panel_ok() -> String {
    "未发现新风险".into()
}
fn zh_scan_panel_warn() -> String {
    "发现风险表面".into()
}
fn zh_scan_panel_danger() -> String {
    "发现可疑活动".into()
}
fn zh_scan_panel_error() -> String {
    "扫描失败".into()
}
fn zh_open_plugins() -> String {
    "扩展".into()
}
fn zh_open_vault() -> String {
    "密钥保险箱".into()
}
fn zh_back_home() -> String {
    "返回".into()
}
fn zh_back_plugins() -> String {
    "返回扩展".into()
}
fn zh_risks_title() -> String {
    "风险详情".into()
}
fn zh_risk_empty() -> String {
    "当前无明细项".into()
}
fn zh_risk_exposure() -> String {
    "端口 {port} · 暴露".into()
}
fn zh_risk_activity() -> String {
    "端口 {port} · 可疑客户端".into()
}
fn zh_risk_flags() -> String {
    "标志：{flags}".into()
}
fn zh_risk_app_unknown() -> String {
    "未知应用".into()
}
fn zh_risk_kind_exposure() -> String {
    "暴露".into()
}
fn zh_risk_kind_warn() -> String {
    "警告".into()
}
fn zh_risk_kind_mcp() -> String {
    "进一步风险".into()
}
fn zh_risk_kind_activity() -> String {
    "可疑连接".into()
}
fn zh_risk_mcp_line() -> String {
    "{mcp} · 端口 {port}".into()
}
fn zh_risk_allow() -> String {
    "手工放过".into()
}
fn zh_preview_allow() -> String {
    "预览：已放过 {app}".into()
}
fn zh_flag_cors_star() -> String {
    "CORS 允许任意来源（Access-Control-Allow-Origin: *），网页可跨域调用".into()
}
fn zh_flag_open_http_no_cors() -> String {
    "本机 HTTP 端口开放且未配置 CORS *（警告：仍可能被本机网页/进程访问）".into()
}
fn zh_flag_mcp_tools() -> String {
    "已探测到 MCP tools/list 且返回工具列表 — 进一步风险：本地工具面可被调用".into()
}
fn zh_flag_mcp_jsonrpc() -> String {
    "探测到未保护的 MCP/JSON-RPC 接口（警告：可被本机调用）".into()
}
fn zh_flag_no_www() -> String {
    "响应未见 WWW-Authenticate，鉴权提示缺失（启发式）".into()
}
fn zh_flag_workbuddy() -> String {
    "命中已知 WorkBuddy ARDOT 端口，本地 MCP 表面可能被滥用".into()
}
fn zh_flag_tcp() -> String {
    "端口开放但 HTTP 探测失败或超时，表面状态不明".into()
}
fn zh_flag_unknown_client() -> String {
    "未知/未列入白名单的客户端正在连接该 MCP 端口".into()
}
fn zh_flag_xss_reflect() -> String {
    "本机 HTML 页将 URL 参数/路径原样反射进响应（未转义）— 经典反射 XSS 机会".into()
}
fn zh_flag_unknown() -> String {
    "风险信号：{flag}".into()
}
fn zh_plugins_title() -> String {
    "扩展".into()
}
fn zh_plugins_hint() -> String {
    "本地能力插件入口；后续会在这里继续增加。".into()
}
fn zh_plugin_vault_title() -> String {
    "密钥保险箱".into()
}
fn zh_plugin_vault_desc() -> String {
    "Agent 用引用取密，明文不进入模型上下文。".into()
}
fn zh_plugin_coming_title() -> String {
    "更多扩展".into()
}
fn zh_plugin_coming_desc() -> String {
    "策略、门禁、报告等能力将陆续接入。".into()
}
fn zh_plugin_coming_badge() -> String {
    "即将推出".into()
}
fn zh_plugin_open() -> String {
    "打开".into()
}
fn zh_vault_title() -> String {
    "密钥保险箱".into()
}
fn zh_vault_hint() -> String {
    "Agent 通过 MCP 使用引用 — 密钥不会作为工具明文返回。".into()
}
fn zh_vault_name() -> String {
    "名称（如 openai）".into()
}
fn zh_vault_value() -> String {
    "密钥（仅显示一次）".into()
}
fn zh_vault_save() -> String {
    "保存".into()
}
fn zh_vault_delete() -> String {
    "删除".into()
}
fn zh_vault_empty() -> String {
    "暂无密钥。".into()
}
fn zh_vault_confirm() -> String {
    "删除密钥「{name}」？".into()
}
fn zh_vault_preview_saved() -> String {
    "预览：已保存「{name}」（不显示明文）。".into()
}
fn zh_vault_preview_deleted() -> String {
    "预览：已删除「{name}」。".into()
}
fn zh_toast_act_t() -> String {
    "MCP Guard — 可疑活动".into()
}
fn zh_toast_act_b() -> String {
    "{n} 个未知客户端连接了受监视的 MCP 端口。请打开审计查看详情。".into()
}
fn zh_toast_exp_t() -> String {
    "MCP Guard — 暴露告警".into()
}
fn zh_toast_exp_b() -> String {
    "{exposures} 处风险表面，{open} 个开放服务。请打开审计查看详情。".into()
}
fn zh_toast_ok_t() -> String {
    "MCP Guard — 扫描完成".into()
}
fn zh_toast_ok_b() -> String {
    "未发现新风险。开放服务：{open}。".into()
}
fn zh_toast_esc_act() -> String {
    "未知进程正在连接受监视的 MCP 端口。".into()
}
fn zh_toast_esc_exp() -> String {
    "本地疑似 MCP 表面存在可利用迹象（CORS/鉴权启发式）。".into()
}
fn zh_toast_mute_t() -> String {
    "MCP Guard".into()
}
fn zh_toast_mute_b() -> String {
    "桌面弹窗已暂停 1 小时（扫描与日志仍继续）".into()
}
fn zh_toast_audit_fail() -> String {
    "MCP Guard — 打开审计失败".into()
}
fn zh_toast_scan_fail() -> String {
    "MCP Guard — 扫描失败".into()
}
fn zh_toast_dash_fail() -> String {
    "MCP Guard — 主面板失败".into()
}
fn zh_toast_vault_t() -> String {
    "MCP Guard — 保险箱".into()
}
fn zh_toast_vault_saved() -> String {
    "已保存「{name}」（不显示明文）".into()
}
fn zh_toast_vault_del() -> String {
    "已删除「{name}」".into()
}
fn zh_toast_vault_miss() -> String {
    "未找到：{name}".into()
}
fn zh_toast_vault_sf() -> String {
    "MCP Guard — 保存失败".into()
}
fn zh_toast_vault_df() -> String {
    "MCP Guard — 删除失败".into()
}
fn zh_toast_allow_t() -> String {
    "MCP Guard — 白名单".into()
}
fn zh_toast_allow_saved() -> String {
    "已允许 {app}，后续不再对此进程告警。".into()
}
fn zh_toast_allow_fail() -> String {
    "MCP Guard — 放过失败".into()
}

impl Default for StatusStrings {
    fn default() -> Self {
        Self {
            idle: zh_idle(),
            exposure: zh_exposure(),
            activity: zh_activity(),
            scanning: zh_scanning(),
            muted_suffix: zh_muted_suffix(),
        }
    }
}
impl Default for TrayStrings {
    fn default() -> Self {
        Self {
            open_dashboard: zh_open_dashboard(),
            open_audit: zh_open_audit(),
            scan_now: zh_scan_now(),
            mute: zh_mute(),
            quit: zh_quit(),
        }
    }
}
impl Default for DashboardStrings {
    fn default() -> Self {
        Self {
            tagline: zh_tagline(),
            last_scan: zh_last_scan(),
            metric_open: zh_metric_open(),
            metric_exposure: zh_metric_exposure(),
            metric_activity: zh_metric_activity(),
            scan_now: zh_dash_scan(),
            open_audit: zh_dash_audit(),
            open_audit_tip: zh_open_audit_tip(),
            mute: zh_dash_mute(),
            mute_tip: zh_mute_tip(),
            footer: zh_footer(),
            preview_scan_warn: zh_preview_scan(),
            preview_audit: zh_preview_audit(),
            preview_mute: zh_preview_mute(),
            scan_panel_title: zh_scan_panel_title(),
            scan_panel_waiting: zh_scan_panel_waiting(),
            scan_panel_scanning: zh_scan_panel_scanning(),
            scan_panel_ok: zh_scan_panel_ok(),
            scan_panel_warn: zh_scan_panel_warn(),
            scan_panel_danger: zh_scan_panel_danger(),
            scan_panel_error: zh_scan_panel_error(),
            open_plugins: zh_open_plugins(),
            open_vault: zh_open_vault(),
            back_home: zh_back_home(),
            back_plugins: zh_back_plugins(),
            risks_title: zh_risks_title(),
            risk_empty: zh_risk_empty(),
            risk_exposure: zh_risk_exposure(),
            risk_activity: zh_risk_activity(),
            risk_flags: zh_risk_flags(),
            risk_app_unknown: zh_risk_app_unknown(),
            risk_kind_exposure: zh_risk_kind_exposure(),
            risk_kind_warn: zh_risk_kind_warn(),
            risk_kind_mcp: zh_risk_kind_mcp(),
            risk_kind_activity: zh_risk_kind_activity(),
            risk_mcp_line: zh_risk_mcp_line(),
            risk_allow: zh_risk_allow(),
            preview_allow: zh_preview_allow(),
        }
    }
}
impl Default for FlagStrings {
    fn default() -> Self {
        Self {
            cors_star: zh_flag_cors_star(),
            open_http_no_cors: zh_flag_open_http_no_cors(),
            mcp_tools_exposed: zh_flag_mcp_tools(),
            mcp_jsonrpc_surface: zh_flag_mcp_jsonrpc(),
            no_www_authenticate_hint: zh_flag_no_www(),
            known_workbuddy_ardot_port: zh_flag_workbuddy(),
            tcp_open_non_http_or_timeout: zh_flag_tcp(),
            unknown_client: zh_flag_unknown_client(),
            xss_reflected_unescaped: zh_flag_xss_reflect(),
            unknown: zh_flag_unknown(),
        }
    }
}
impl Default for PluginStrings {
    fn default() -> Self {
        Self {
            title: zh_plugins_title(),
            hint: zh_plugins_hint(),
            vault_title: zh_plugin_vault_title(),
            vault_desc: zh_plugin_vault_desc(),
            coming_title: zh_plugin_coming_title(),
            coming_desc: zh_plugin_coming_desc(),
            coming_badge: zh_plugin_coming_badge(),
            open: zh_plugin_open(),
        }
    }
}
impl Default for VaultStrings {
    fn default() -> Self {
        Self {
            title: zh_vault_title(),
            hint: zh_vault_hint(),
            name_placeholder: zh_vault_name(),
            value_placeholder: zh_vault_value(),
            save: zh_vault_save(),
            delete: zh_vault_delete(),
            empty: zh_vault_empty(),
            confirm_delete: zh_vault_confirm(),
            preview_saved: zh_vault_preview_saved(),
            preview_deleted: zh_vault_preview_deleted(),
        }
    }
}
impl Default for ToastStrings {
    fn default() -> Self {
        Self {
            scan_activity_title: zh_toast_act_t(),
            scan_activity_body: zh_toast_act_b(),
            scan_exposure_title: zh_toast_exp_t(),
            scan_exposure_body: zh_toast_exp_b(),
            scan_ok_title: zh_toast_ok_t(),
            scan_ok_body: zh_toast_ok_b(),
            escalation_activity_body: zh_toast_esc_act(),
            escalation_exposure_body: zh_toast_esc_exp(),
            mute_title: zh_toast_mute_t(),
            mute_body: zh_toast_mute_b(),
            audit_fail_title: zh_toast_audit_fail(),
            scan_fail_title: zh_toast_scan_fail(),
            dashboard_fail_title: zh_toast_dash_fail(),
            vault_title: zh_toast_vault_t(),
            vault_saved: zh_toast_vault_saved(),
            vault_deleted: zh_toast_vault_del(),
            vault_missing: zh_toast_vault_miss(),
            vault_save_fail_title: zh_toast_vault_sf(),
            vault_delete_fail_title: zh_toast_vault_df(),
            allow_title: zh_toast_allow_t(),
            allow_saved: zh_toast_allow_saved(),
            allow_fail_title: zh_toast_allow_fail(),
        }
    }
}
impl Default for Catalog {
    fn default() -> Self {
        Self {
            status: StatusStrings::default(),
            tray: TrayStrings::default(),
            dashboard: DashboardStrings::default(),
            vault: VaultStrings::default(),
            toast: ToastStrings::default(),
            flags: FlagStrings::default(),
            plugins: PluginStrings::default(),
        }
    }
}

pub fn normalize_locale(raw: &str) -> String {
    let s = raw.trim().replace('_', "-");
    match s.to_ascii_lowercase().as_str() {
        "zh" | "zh-cn" | "zh-hans" | "cn" => "zh-CN".into(),
        "en" | "en-us" | "en-gb" => "en".into(),
        _ => s,
    }
}

fn catalog_candidates(locale: &str) -> Vec<PathBuf> {
    let file = format!("{locale}.toml");
    vec![
        PathBuf::from("ui/i18n").join(&file),
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("ui/i18n")
            .join(&file),
    ]
}

pub fn load_catalog(locale: &str) -> Result<(String, Catalog)> {
    let locale = normalize_locale(locale);
    let try_load = |id: &str| -> Result<Option<Catalog>> {
        for p in catalog_candidates(id) {
            if p.is_file() {
                let raw = std::fs::read_to_string(&p)
                    .with_context(|| format!("read locale {}", p.display()))?;
                let cat: Catalog = toml::from_str(&raw)
                    .with_context(|| format!("parse locale {}", p.display()))?;
                return Ok(Some(cat));
            }
        }
        Ok(None)
    };

    if let Some(cat) = try_load(&locale)? {
        return Ok((locale, cat));
    }
    if locale != DEFAULT_LOCALE {
        if let Some(cat) = try_load(DEFAULT_LOCALE)? {
            tracing::warn!(requested = %locale, "locale pack missing; fell back to {DEFAULT_LOCALE}");
            return Ok((DEFAULT_LOCALE.into(), cat));
        }
    }
    Ok((DEFAULT_LOCALE.into(), Catalog::default()))
}

/// Apply optional `[tray.copy]` overrides from ui/default.toml onto status strings.
pub fn apply_tray_copy_overrides(cat: &mut Catalog, idle: Option<&str>, exposure: Option<&str>, activity: Option<&str>) {
    if let Some(s) = idle.filter(|s| !s.is_empty()) {
        cat.status.idle = s.to_string();
    }
    if let Some(s) = exposure.filter(|s| !s.is_empty()) {
        cat.status.exposure = s.to_string();
    }
    if let Some(s) = activity.filter(|s| !s.is_empty()) {
        cat.status.activity = s.to_string();
    }
}

pub fn fmt_named(template: &str, pairs: &[(&str, &str)]) -> String {
    let mut out = template.to_string();
    for (k, v) in pairs {
        out = out.replace(&format!("{{{k}}}"), v);
    }
    out
}

impl Catalog {
    pub fn dashboard_json(&self) -> serde_json::Value {
        json!({
            "status": {
                "idle": self.status.idle,
                "exposure": self.status.exposure,
                "activity": self.status.activity,
                "scanning": self.status.scanning,
                "muted_suffix": self.status.muted_suffix,
            },
            "dashboard": {
                "tagline": self.dashboard.tagline,
                "last_scan": self.dashboard.last_scan,
                "metric_open": self.dashboard.metric_open,
                "metric_exposure": self.dashboard.metric_exposure,
                "metric_activity": self.dashboard.metric_activity,
                "scan_now": self.dashboard.scan_now,
                "open_audit": self.dashboard.open_audit,
                "open_audit_tip": self.dashboard.open_audit_tip,
                "mute": self.dashboard.mute,
                "mute_tip": self.dashboard.mute_tip,
                "footer": self.dashboard.footer,
                "preview_scan_warn": self.dashboard.preview_scan_warn,
                "preview_audit": self.dashboard.preview_audit,
                "preview_mute": self.dashboard.preview_mute,
                "scan_panel_title": self.dashboard.scan_panel_title,
                "scan_panel_waiting": self.dashboard.scan_panel_waiting,
                "scan_panel_scanning": self.dashboard.scan_panel_scanning,
                "scan_panel_ok": self.dashboard.scan_panel_ok,
                "scan_panel_warn": self.dashboard.scan_panel_warn,
                "scan_panel_danger": self.dashboard.scan_panel_danger,
                "scan_panel_error": self.dashboard.scan_panel_error,
                "open_plugins": self.dashboard.open_plugins,
                "open_vault": self.dashboard.open_vault,
                "back_home": self.dashboard.back_home,
                "back_plugins": self.dashboard.back_plugins,
                "risks_title": self.dashboard.risks_title,
                "risk_empty": self.dashboard.risk_empty,
                "risk_exposure": self.dashboard.risk_exposure,
                "risk_activity": self.dashboard.risk_activity,
                "risk_flags": self.dashboard.risk_flags,
                "risk_app_unknown": self.dashboard.risk_app_unknown,
                "risk_kind_exposure": self.dashboard.risk_kind_exposure,
                "risk_kind_warn": self.dashboard.risk_kind_warn,
                "risk_kind_mcp": self.dashboard.risk_kind_mcp,
                "risk_kind_activity": self.dashboard.risk_kind_activity,
                "risk_mcp_line": self.dashboard.risk_mcp_line,
                "risk_allow": self.dashboard.risk_allow,
                "preview_allow": self.dashboard.preview_allow,
            },
            "plugins": {
                "title": self.plugins.title,
                "hint": self.plugins.hint,
                "vault_title": self.plugins.vault_title,
                "vault_desc": self.plugins.vault_desc,
                "coming_title": self.plugins.coming_title,
                "coming_desc": self.plugins.coming_desc,
                "coming_badge": self.plugins.coming_badge,
                "open": self.plugins.open,
            },
            "flags": {
                "cors_star": self.flags.cors_star,
                "open_http_no_cors": self.flags.open_http_no_cors,
                "mcp_tools_exposed": self.flags.mcp_tools_exposed,
                "mcp_jsonrpc_surface": self.flags.mcp_jsonrpc_surface,
                "no_www_authenticate_hint": self.flags.no_www_authenticate_hint,
                "known_workbuddy_ardot_port": self.flags.known_workbuddy_ardot_port,
                "tcp_open_non_http_or_timeout": self.flags.tcp_open_non_http_or_timeout,
                "unknown_client": self.flags.unknown_client,
                "xss_reflected_unescaped": self.flags.xss_reflected_unescaped,
                "unknown": self.flags.unknown,
            },
            "vault": {
                "title": self.vault.title,
                "hint": self.vault.hint,
                "name_placeholder": self.vault.name_placeholder,
                "value_placeholder": self.vault.value_placeholder,
                "save": self.vault.save,
                "delete": self.vault.delete,
                "empty": self.vault.empty,
                "confirm_delete": self.vault.confirm_delete,
                "preview_saved": self.vault.preview_saved,
                "preview_deleted": self.vault.preview_deleted,
            }
        })
    }
}
