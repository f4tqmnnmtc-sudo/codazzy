use crate::config::CONFIG;
use std::fmt::Write;

pub struct FluxQuery {
    bucket: String,
    range: String,
    filters: Vec<String>,
    group_cols: Option<Vec<String>>,
    keep_cols: Option<Vec<String>>,
    agg: Option<&'static str>,
    imports: Vec<&'static str>,
    suffix: Vec<String>,
}

impl FluxQuery {
    pub fn new() -> Self {
        Self {
            bucket: CONFIG.influx_bucket.clone(),
            range: "5m".into(),
            filters: Vec::with_capacity(4),
            group_cols: None,
            keep_cols: None,
            agg: None,
            imports: vec![],
            suffix: Vec::with_capacity(2),
        }
    }

    pub fn bucket(mut self, b: &str) -> Self { self.bucket = b.into(); self }
    pub fn range(mut self, r: &str) -> Self { self.range = r.into(); self }

    pub fn range_future(mut self, start: &str, stop: &str) -> Self {
        self.imports.push(r#"import "date""#);
        self.range = format!("{start}, stop: {stop}");
        self
    }

    pub fn measurement(mut self, m: &str) -> Self {
        self.filters.push(format!(r#"r._measurement == "{m}""#));
        self
    }

    pub fn measurements(mut self, ms: &[&str]) -> Self {
        let cond: String = ms.iter().map(|m| format!(r#"r._measurement == "{m}""#)).collect::<Vec<_>>().join(" or ");
        self.filters.push(format!("({cond})"));
        self
    }

    pub fn field(mut self, f: &str) -> Self {
        self.filters.push(format!(r#"r._field == "{f}""#));
        self
    }

    pub fn fields(mut self, fs: &[&str]) -> Self {
        let cond: String = fs.iter().map(|f| format!(r#"r._field == "{f}""#)).collect::<Vec<_>>().join(" or ");
        self.filters.push(format!("({cond})"));
        self
    }

    pub fn tag(mut self, k: &str, v: &str) -> Self {
        self.filters.push(format!(r#"r.{k} == "{v}""#));
        self
    }

    pub fn tag_or(mut self, k: &str, vs: &[&str]) -> Self {
        let cond: String = vs.iter().map(|v| format!(r#"r.{k} == "{v}""#)).collect::<Vec<_>>().join(" or ");
        self.filters.push(format!("({cond})"));
        self
    }

    pub fn component(mut self, c: &str) -> Self {
        self.filters.push(format!(r#"r.component == "{c}""#));
        self
    }

    pub fn component_or(mut self, cs: &[&str]) -> Self {
        let cond: String = cs.iter().map(|c| format!(r#"r.component == "{c}""#)).collect::<Vec<_>>().join(" or ");
        self.filters.push(format!("({cond})"));
        self
    }

    pub fn node_id(mut self, nid: &str) -> Self {
        self.filters.push(format!(r#"(r.node_id == "{nid}" or r.agent_id == "{nid}")"#));
        self
    }

    pub fn node_id_opt(self, nid: Option<&str>) -> Self {
        match nid { Some(n) => self.node_id(n), None => self }
    }

    pub fn component_opt(self, c: Option<&str>) -> Self {
        match c { Some(comp) => self.component(comp), None => self }
    }

    pub fn metric_type(mut self, mt: &str) -> Self {
        self.filters.push(format!(r#"r.metric_type == "{mt}""#));
        self
    }

    pub fn exclude_virtual_ifaces(mut self) -> Self {
        self.filters.push(r#"r.component !~ /^lo_/ and r.component !~ /^docker/ and r.component !~ /^veth/ and r.component !~ /^br-/"#.into());
        self
    }

    pub fn component_regex(mut self, pat: &str) -> Self {
        self.filters.push(format!(r#"r.component =~ /{pat}/"#));
        self
    }

    pub fn raw_filter(mut self, f: &str) -> Self { self.filters.push(f.into()); self }

    pub fn group(mut self, cols: &[&str]) -> Self {
        self.group_cols = Some(cols.iter().map(|s| (*s).into()).collect());
        self
    }

    pub fn keep(mut self, cols: &[&str]) -> Self {
        self.keep_cols = Some(cols.iter().map(|s| (*s).into()).collect());
        self
    }

    pub fn last(mut self) -> Self { self.agg = Some("last()"); self }
    pub fn mean(mut self) -> Self { self.agg = Some("mean()"); self }
    pub fn count(mut self) -> Self { self.agg = Some("count()"); self }

    pub fn derivative(mut self, unit: &str, non_neg: bool) -> Self {
        let nn = if non_neg { ", nonNegative: true" } else { "" };
        self.suffix.push(format!("|> derivative(unit: {unit}{nn})"));
        self
    }

    pub fn aggregate_window(mut self, every: &str, func: &str) -> Self {
        self.suffix.push(format!("|> aggregateWindow(every: {every}, fn: {func})"));
        self
    }

    pub fn pivot(mut self) -> Self {
        self.suffix.push(r#"|> pivot(rowKey:["_time"], columnKey: ["_field"], valueColumn: "_value")"#.into());
        self
    }

    pub fn sort(mut self, cols: &[&str], desc: bool) -> Self {
        let cols_str: String = cols.iter().map(|c| format!(r#""{c}""#)).collect::<Vec<_>>().join(", ");
        self.suffix.push(if desc {
            format!("|> sort(columns: [{cols_str}], desc: true)")
        } else {
            format!("|> sort(columns: [{cols_str}])")
        });
        self
    }

    pub fn limit(mut self, n: usize) -> Self {
        self.suffix.push(format!("|> limit(n: {n})"));
        self
    }

    pub fn build(self) -> String {
        let cap = 256 + self.filters.len() * 48 + self.suffix.len() * 32;
        let mut q = String::with_capacity(cap);

        for imp in &self.imports { let _ = writeln!(q, "{imp}"); }

        let _ = write!(q, r#"from(bucket: "{}") |> range(start: -{})"#, self.bucket, self.range);

        for f in &self.filters { let _ = write!(q, "\n  |> filter(fn: (r) => {f})"); }

        if let Some(ref cols) = self.group_cols {
            let cols_str: String = cols.iter().map(|c| format!(r#""{c}""#)).collect::<Vec<_>>().join(", ");
            let _ = write!(q, "\n  |> group(columns: [{cols_str}])");
        }

        if let Some(agg) = self.agg { let _ = write!(q, "\n  |> {agg}"); }

        for s in &self.suffix { let _ = write!(q, "\n  {s}"); }

        if let Some(ref cols) = self.keep_cols {
            let cols_str: String = cols.iter().map(|c| format!(r#""{c}""#)).collect::<Vec<_>>().join(", ");
            let _ = write!(q, "\n  |> keep(columns: [{cols_str}])");
        }

        q
    }
}

impl Default for FluxQuery { fn default() -> Self { Self::new() } }

pub fn range_to_mins(r: &str) -> i64 {
    // Parseo de rangos tipo "5m", "1h", "24h" a minutos
    let s = r.trim_start_matches('-');
    let (num, unit) = s.split_at(s.len().saturating_sub(1));

    num.parse::<i64>().ok().map(|n| match unit {
        "m" => n,
        "h" => n * 60,
        "d" => n * 1440,
        _ => 15,
    }).unwrap_or(15)
}
