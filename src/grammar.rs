//! The phrase grammar. No sentence womm says is stored whole: a small
//! context-free grammar defines the shapes, NoNet (or any `Chooser`) picks every
//! production, and real words from the user's plan get spliced in.

use crate::rng::Chooser;

/// What the grammar knows about the situation.
#[derive(Default, Clone)]
pub struct Ctx {
    pub words: Vec<String>,
    pub kludges: usize,
    pub topic: String,
}

impl Ctx {
    pub fn from_text(text: &str) -> Ctx {
        Ctx { words: plan_words(text), ..Ctx::default() }
    }
}

/// Interesting words: alphabetic, 4+ letters, longest (rarest-looking) first.
pub fn plan_words(text: &str) -> Vec<String> {
    let mut words: Vec<String> = text
        .split(|c: char| !c.is_alphanumeric() && c != '-')
        .map(|w| w.trim_matches('-').to_lowercase())
        .filter(|w| w.chars().count() >= 4 && w.chars().all(|c| c.is_alphabetic() || c == '-'))
        .collect();
    words.sort_by(|a, b| b.chars().count().cmp(&a.chars().count()).then(a.cmp(b)));
    words.dedup();
    words.truncate(24);
    words
}

type Rule = (&'static str, &'static [&'static str]);

const RULES: &[Rule] = &[
    // ---- shared vocabulary ----
    ("ADJ", &[
        "unscoped", "load-bearing", "aspirational", "legacy", "synergistic", "cloud-native",
        "deprecated", "mission-critical", "artisanal", "undocumented", "blockchain-adjacent",
        "enterprise-grade", "vibe-coded", "haunted", "optimistic", "unreviewed", "quarterly",
        "hand-rolled", "flaky", "bespoke",
    ]),
    ("NOUN", &[
        "stakeholder", "microservice", "deadline", "intern", "sprint", "roadmap", "dependency",
        "standup", "manager", "monorepo", "hotfix", "retro", "kludge", "YAML file", "build server",
        "Jira ticket", "cache", "consultant", "feature flag", "linter",
    ]),
    ("NOUNS", &[
        "stakeholders", "microservices", "deadlines", "interns", "sprints", "dependencies",
        "standups", "managers", "hotfixes", "kludges", "YAML files", "feature flags", "meetings",
        "story points", "TODOs",
    ]),
    ("GERUND", &[
        "Inlining", "Unrolling", "Eliminating", "Hoisting", "Deduplicating", "Vectorising",
        "Refactoring", "Monetising", "Containerising", "Pivoting", "Aligning", "Descoping",
        "Leveraging", "Sunsetting", "Re-platforming",
    ]),
    ("TIME", &[
        "on a Tuesday", "at 2am", "last sprint", "in staging", "before the reorg", "in 2014",
        "during the offsite", "on a Friday", "after lunch", "in a previous life",
    ]),
    // ---- compiler warnings ----
    ("WARN_BODY", &[
        "{SUBJECT} {VERB} {OBJECT}{CONSEQ}",
        "{SUBJECT} {VERB} {OBJECT}{CONSEQ}",
        "{SUBJECT} is {ADJ}{CONSEQ}",
    ]),
    ("SUBJECT", &["the word \"{PLANWORD}\"", "the word \"{PLANWORD}\"", "this line", "your plan", "the {NOUN}"]),
    ("VERB", &[
        "implies", "contains", "depends on", "contradicts", "assumes", "was last seen near",
        "quietly requires", "is blocked by", "has opinions about", "reinvents",
    ]),
    ("OBJECT", &[
        "a deadline", "a {ADJ} {NOUN}", "\"{PLANWORD}\"", "the previous line", "the machine",
        "{N} {NOUNS}", "a meeting",
    ]),
    ("CONSEQ", &[
        "", "; estimate multiplied by {N}", "; this is fine", "; consider not", "; kludge count +{N}",
        "; noted", "; this will be a problem {TIME}", "; scheduling a retro",
    ]),
    // ---- excuses ----
    ("EXCUSE", &[
        "works in staging", "did you try cargo clean", "it passed CI {TIME}", "probably DNS",
        "that is a known issue (unknown to you)", "cannot reproduce", "it's a caching issue",
        "the {NOUN} did it", "upstream {NOUN} bug", "the {ADJ} {NOUN} is to blame",
        "it compiled {TIME}", "have you tried turning the {NOUN} off and on again",
        "works on my machine", "that's a feature, not a {NOUN}", "it's {ADJ}, give it a minute",
    ]),
    // ---- -O passes ----
    ("PASS", &["{GERUND} {NOUNS}", "{GERUND} {ADJ} {NOUNS}", "{GERUND} the {NOUN}"]),
    ("PASS_RESULT", &["ok", "ok", "ok (barely)", "ok (allegedly)", "skipped (it looked busy)", "ok?"]),
    // ---- garbage collector ----
    ("GC", &[
        "collecting kludge #{KLUDGE}... it had a family",
        "sweeping {N} {ADJ} {NOUNS} under the rug",
        "freeing the {NOUN}; it did not want to go",
        "compacting {NOUNS} into a smaller sadness",
        "deleting \"{PLANWORD}\" (just the copy)",
        "returning {N} bytes to the {NOUN}",
        "unreferencing the {ADJ} {NOUN}",
        "emptying the trash. the trash is also trash",
    ]),
    // ---- kludge rot ----
    ("ROT_STALE", &["smells like legacy", "is starting to turn", "has a {ADJ} aftertaste", "is technically still edible"]),
    ("ROT_OLD", &["has developed opinions", "now requires a {NOUN}", "is unionising", "remembers {TIME}"]),
    ("ROT_ANCIENT", &["predates the current management", "is now load-bearing", "has its own {NOUN}", "is older than the {NOUN}"]),
    // ---- review comments ----
    ("REVIEW_OPENER", &["nit:", "question:", "suggestion:", "blocking:", "praise?:", "concern:"]),
    ("REVIEW_TAIL", &[
        "who is this for", "respectfully, why", "this is {ADJ}", "consider a {NOUN}",
        "please add a test (we both know you won't)", "see previous comment", "bold",
        "I'd love to understand the thinking here", "this will not age well",
    ]),
    // ---- moods / horoscope ----
    ("HOROSCOPE", &[
        "today favours {ADJ} {NOUNS}; avoid the {NOUN}",
        "a {NOUN} from your past returns {TIME}",
        "expect {N} {NOUNS} and one honest compiler",
        "your lucky opcode is {OPCODE}",
        "mercury is in {ADJ} retrograde; ship nothing",
    ]),
    ("OPCODE", &["GLYPH", "LOOP", "EXCUSE", "MIRROR", "SKIP", "STRIPE", "HALT"]),
    // ---- womm ask ----
    ("ASK_STEP", &[
        "considering your machine", "considering my machine", "noticing they are different",
        "consulting {N} {NOUNS}", "weighing \"{TOPIC}\" against the roadmap",
        "checking whether \"{TOPIC}\" is in scope (it is not)", "asking the {NOUN}",
        "re-reading the question {TIME}",
    ]),
    ("NO", &["No.", "No.", "no", "Absolutely not.", "Not on this machine.", "Ask again after a reboot.", "That's a no from me (NoNet)."]),
    ("NO_REASON", &[
        "the word \"{TOPIC}\" implies a deadline", "it did not work on my machine",
        "the {NOUN} said no first", "out of scope for this sprint", "that would require a {NOUN}",
    ]),
    // ---- womm why ----
    ("WHY", &[
        "the {STAGE} stage exists because the {NOUN} asked for documentation",
        "RSA-2048 was chosen because 4096 felt like showing off",
        "the {STAGE} stage exists because {N} {NOUNS} could not agree",
        "kludge files are TOML because YAML was {ADJ}",
        "the {STAGE} stage was added {TIME} and nobody has dared remove it",
        "NoNet has 833 parameters because 834 failed the {ADJ} review",
    ]),
    ("STAGE", &["parse", "build", "parsy-builder", "garbage collector", "StableHLO", "markdown"]),
    // ---- standup ----
    ("BLOCKER", &["the machine", "the {NOUN}", "{N} {ADJ} {NOUNS}", "my own plan", "NoNet (it said no)"]),
    // ---- machine lock ----
    ("PROTEST", &["under protest", "against my better judgement", "with visible reluctance", "but I'm telling the {NOUN}"]),
];

fn rule(name: &str) -> &'static [&'static str] {
    RULES.iter().find(|(n, _)| *n == name).map(|(_, alts)| *alts).unwrap_or(&[])
}

/// Expand a start symbol into a sentence.
pub fn say(sym: &str, ctx: &Ctx, c: &mut dyn Chooser) -> String {
    expand(sym, ctx, c, 0)
}

fn expand(sym: &str, ctx: &Ctx, c: &mut dyn Chooser, depth: usize) -> String {
    match sym {
        "PLANWORD" => {
            if ctx.words.is_empty() {
                return "synergy".into();
            }
            // Bias toward the longer words at the front of the list.
            let n = ctx.words.len();
            let i = c.pick(n).min(c.pick(n));
            return ctx.words[i].clone();
        }
        "TOPIC" => return if ctx.topic.is_empty() { "this".into() } else { ctx.topic.clone() },
        "N" => return (2 + c.pick(8)).to_string(),
        "KLUDGE" => return c.pick(ctx.kludges.max(4)).to_string(),
        _ => {}
    }
    let alts = rule(sym);
    if alts.is_empty() || depth > 8 {
        return sym.to_lowercase();
    }
    let template = alts[c.pick(alts.len())];
    let mut out = String::new();
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let close = rest[open..].find('}').map(|i| i + open).unwrap_or(rest.len() - 1);
        out.push_str(&expand(&rest[open + 1..close], ctx, c, depth + 1));
        rest = &rest[close + 1..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::Rng;

    #[test]
    fn every_placeholder_resolves() {
        let builtin = ["PLANWORD", "TOPIC", "N", "KLUDGE"];
        for (name, alts) in RULES {
            for alt in *alts {
                let mut rest = *alt;
                while let Some(open) = rest.find('{') {
                    let close = rest[open..].find('}').unwrap() + open;
                    let sym = &rest[open + 1..close];
                    assert!(
                        builtin.contains(&sym) || !rule(sym).is_empty(),
                        "{name}: unknown symbol {sym}"
                    );
                    rest = &rest[close + 1..];
                }
            }
        }
    }

    #[test]
    fn sentences_quote_the_plan_and_are_deterministic() {
        let ctx = Ctx::from_text("We will leverage blockchain synergy\nShip by Friday");
        assert!(ctx.words.contains(&"blockchain".to_string()));
        let a: Vec<String> = (0..20).map(|_| String::new()).scan(Rng::from_seed(b"k"), |r, _| Some(say("WARN_BODY", &ctx, r))).collect();
        let b: Vec<String> = (0..20).map(|_| String::new()).scan(Rng::from_seed(b"k"), |r, _| Some(say("WARN_BODY", &ctx, r))).collect();
        assert_eq!(a, b);
        assert!(a.iter().all(|s| !s.contains('{')));
    }

    #[test]
    fn lexicon_is_reasonably_large() {
        let total: usize = RULES.iter().map(|(_, a)| a.len()).sum();
        assert!(total >= 200, "only {total} fragments");
    }
}
