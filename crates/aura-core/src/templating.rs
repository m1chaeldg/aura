//! Handlebars and dynamic variable templating engine for Aura.
//! Resolves `{{var}}` syntax across URLs, params, headers, and body payloads,
//! with cascading lookup: Request -> Environment -> Collection -> OS Environment -> Built-in Fakers/Dynamics.

use rand::Rng;
use regex::Regex;
use std::collections::HashMap;
use std::sync::LazyLock;

static VAR_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\{\{([a-zA-Z0-9_$.-]+)\}\}").unwrap());

/// Context for resolving variables in a request
#[derive(Debug, Clone, Default)]
pub struct TemplateContext {
    pub request_vars: HashMap<String, String>,
    pub environment_vars: HashMap<String, String>,
    pub collection_vars: HashMap<String, String>,
}

impl TemplateContext {
    pub fn new() -> Self {
        Self::default()
    }

    /// Resolve a single variable name following the cascade:
    /// Request -> Environment -> Collection -> OS environment ($VAR) -> Built-in dynamic/faker ($...)
    pub fn resolve_variable(&self, key: &str) -> Option<String> {
        // 1. Request variables (highest precedence)
        if let Some(val) = self.request_vars.get(key) {
            return Some(val.clone());
        }

        // 2. Environment variables
        if let Some(val) = self.environment_vars.get(key) {
            return Some(val.clone());
        }

        // 3. Collection variables
        if let Some(val) = self.collection_vars.get(key) {
            return Some(val.clone());
        }

        // 4. Built-in dynamic / faker variables (prefixed with $)
        if let Some(dynamic) = resolve_builtin_dynamic(key) {
            return Some(dynamic);
        }

        // 5. OS Environment variables: if prefixed with $ e.g. $USER, $OS_USER, or directly
        if let Some(stripped) = key.strip_prefix('$') {
            if let Ok(val) = std::env::var(stripped) {
                return Some(val);
            }
        }
        if let Ok(val) = std::env::var(key) {
            return Some(val);
        }

        None
    }

    /// Render a templated string by substituting all `{{...}}` tokens
    pub fn render(&self, template: &str) -> String {
        VAR_REGEX
            .replace_all(template, |caps: &regex::Captures| {
                let var_name = &caps[1];
                if let Some(val) = self.resolve_variable(var_name) {
                    val
                } else {
                    caps[0].to_string()
                }
            })
            .to_string()
    }
}

fn pick_str<'a>(items: &'a [&'a str]) -> &'a str {
    let idx = rand::thread_rng().gen_range(0..items.len());
    items[idx]
}

/// Resolves Postman/Aura built-in dynamic variables (`{{$uuid}}`, `{{$timestamp}}`, `{{$faker.*}}`, etc.).
pub fn resolve_builtin_dynamic(key: &str) -> Option<String> {
    const FIRST_NAMES: &[&str] = &[
        "James", "Mary", "Robert", "Patricia", "John", "Jennifer", "Michael", "Linda",
        "David", "Elizabeth", "William", "Barbara", "Richard", "Susan", "Joseph", "Jessica",
    ];
    const LAST_NAMES: &[&str] = &[
        "Smith", "Johnson", "Williams", "Brown", "Jones", "Garcia", "Miller", "Davis",
        "Rodriguez", "Martinez", "Hernandez", "Lopez", "Gonzalez", "Wilson", "Anderson",
    ];
    const DOMAINS: &[&str] = &["example.com", "test.org", "demo.net", "aura.dev", "mail.com"];
    const CITIES: &[&str] = &[
        "New York", "London", "Tokyo", "Paris", "Berlin", "Sydney", "Toronto", "Singapore",
        "San Francisco", "Dublin",
    ];
    const COUNTRIES: &[&str] = &[
        "United States", "United Kingdom", "Canada", "Germany", "France", "Japan", "Australia",
        "Singapore",
    ];
    const STREETS: &[&str] = &[
        "Main St", "High St", "Maple Ave", "Oak St", "Park Blvd", "Broadway", "Market St",
        "Lakeview Dr",
    ];
    const WORDS: &[&str] = &[
        "agile", "robust", "scalable", "ephemeral", "secure", "distributed", "protocol",
        "payload", "stream", "pipeline", "cluster", "gateway", "latency",
    ];
    const COMPANIES: &[&str] = &[
        "Acme Corp", "Globex", "Initech", "Umbrella", "Cyberdyne", "Massive Dynamic", "Hooli",
        "Pied Piper",
    ];
    const JOB_TITLES: &[&str] = &[
        "Software Engineer", "DevOps Specialist", "Product Manager", "QA Architect",
        "Systems Analyst", "SRE",
    ];

    let mut rng = rand::thread_rng();
    match key {
        "$uuid" | "$guid" | "$faker.string.uuid" | "$faker.datatype.uuid" | "$faker.random.uuid" => {
            Some(uuid::Uuid::new_v4().to_string())
        }
        "$timestamp" => Some(chrono::Utc::now().timestamp().to_string()),
        "$isoTimestamp" | "$faker.date.recent" => Some(chrono::Utc::now().to_rfc3339()),
        "$randomInt" | "$faker.number.int" | "$faker.datatype.number" => {
            Some(rng.gen_range(0..10000).to_string())
        }
        "$randomFloat" | "$faker.number.float" => {
            let val: f64 = rng.gen_range(0.0..100.0);
            Some(format!("{val:.2}"))
        }
        "$randomBool" | "$randomBoolean" | "$faker.datatype.boolean" | "$faker.random.boolean" => {
            Some(rng.gen_bool(0.5).to_string())
        }
        "$randomFirstName" | "$faker.person.firstName" | "$faker.name.firstName" => {
            Some(pick_str(FIRST_NAMES).to_string())
        }
        "$randomLastName" | "$faker.person.lastName" | "$faker.name.lastName" => {
            Some(pick_str(LAST_NAMES).to_string())
        }
        "$randomFullName" | "$faker.person.fullName" | "$faker.name.fullName" => {
            Some(format!("{} {}", pick_str(FIRST_NAMES), pick_str(LAST_NAMES)))
        }
        "$randomJobTitle" | "$faker.person.jobTitle" | "$faker.name.jobTitle" => {
            Some(pick_str(JOB_TITLES).to_string())
        }
        "$randomEmail" | "$faker.internet.email" => Some(format!(
            "{}.{}{}@{}",
            pick_str(FIRST_NAMES).to_lowercase(),
            pick_str(LAST_NAMES).to_lowercase(),
            rng.gen_range(10..999),
            pick_str(DOMAINS)
        )),
        "$randomUserName" | "$faker.internet.userName" => Some(format!(
            "{}_{}",
            pick_str(FIRST_NAMES).to_lowercase(),
            rng.gen_range(100..999)
        )),
        "$randomUrl" | "$faker.internet.url" => {
            Some(format!("https://{}.{}/api/v1", pick_str(WORDS), pick_str(DOMAINS)))
        }
        "$randomIp" | "$faker.internet.ip" => Some(format!(
            "{}.{}.{}.{}",
            rng.gen_range(1..254),
            rng.gen_range(1..254),
            rng.gen_range(1..254),
            rng.gen_range(1..254)
        )),
        "$randomColor" | "$faker.internet.color" => {
            Some(format!("#{:06x}", rng.gen_range(0..0xFFFFFFu32)))
        }
        "$randomAlphaNumeric" | "$faker.string.alphanumeric" | "$faker.random.alphaNumeric" => {
            const CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
            let s: String = (0..8)
                .map(|_| CHARS[rng.gen_range(0..CHARS.len())] as char)
                .collect();
            Some(s)
        }
        "$randomCity" | "$faker.location.city" | "$faker.address.city" => {
            Some(pick_str(CITIES).to_string())
        }
        "$randomCountry" | "$faker.location.country" | "$faker.address.country" => {
            Some(pick_str(COUNTRIES).to_string())
        }
        "$randomStreetAddress" | "$faker.location.streetAddress" | "$faker.address.streetAddress" => {
            Some(format!("{} {}", rng.gen_range(100..9999), pick_str(STREETS)))
        }
        "$randomCompany" | "$faker.company.name" => Some(pick_str(COMPANIES).to_string()),
        "$randomPrice" | "$faker.commerce.price" => {
            Some(format!("{}.{:02}", rng.gen_range(5..500), rng.gen_range(10..99)))
        }
        "$randomProductName" | "$faker.commerce.productName" => {
            Some(format!("{} {}", pick_str(WORDS), pick_str(WORDS)))
        }
        "$randomWord" | "$faker.lorem.word" | "$faker.random.word" => {
            Some(pick_str(WORDS).to_string())
        }
        "$randomWords" | "$faker.lorem.words" | "$faker.random.words" => Some(format!(
            "{} {} {}",
            pick_str(WORDS),
            pick_str(WORDS),
            pick_str(WORDS)
        )),
        "$randomSentence" | "$faker.lorem.sentence" => Some(format!(
            "{} {} {} {}.",
            pick_str(WORDS),
            pick_str(WORDS),
            pick_str(WORDS),
            pick_str(WORDS)
        )),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_builtin_dynamic_and_faker_tokens() {
        let ctx = TemplateContext::new();
        let rendered = ctx.render("id={{$uuid}}&ts={{$timestamp}}&email={{$faker.internet.email}}&n={{$randomInt}}");
        assert!(!rendered.contains("{{$uuid}}"));
        assert!(!rendered.contains("{{$timestamp}}"));
        assert!(!rendered.contains("{{$faker.internet.email}}"));
        assert!(!rendered.contains("{{$randomInt}}"));
        assert!(rendered.contains('@'));
    }

    #[test]
    fn request_var_overrides_builtin_if_explicitly_defined() {
        let mut ctx = TemplateContext::new();
        ctx.request_vars.insert("$uuid".to_string(), "fixed-uuid".to_string());
        assert_eq!(ctx.render("{{$uuid}}"), "fixed-uuid");
    }
}
