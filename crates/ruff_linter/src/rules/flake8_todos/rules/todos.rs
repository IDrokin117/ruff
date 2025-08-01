use std::sync::LazyLock;

use regex::RegexSet;

use ruff_macros::{ViolationMetadata, derive_message_formats};
use ruff_python_trivia::CommentRanges;
use ruff_text_size::{TextLen, TextRange, TextSize};

use crate::Locator;
use crate::checkers::ast::LintContext;
use crate::directives::{TodoComment, TodoDirective, TodoDirectiveKind};
use crate::{AlwaysFixableViolation, Edit, Fix, Violation};

/// ## What it does
/// Checks that a TODO comment is labelled with "TODO".
///
/// ## Why is this bad?
/// Ambiguous tags reduce code visibility and can lead to dangling TODOs.
/// For example, if a comment is tagged with "FIXME" rather than "TODO", it may
/// be overlooked by future readers.
///
/// Note that this rule will only flag "FIXME" and "XXX" tags as incorrect.
///
/// ## Example
/// ```python
/// # FIXME(ruff): this should get fixed!
/// ```
///
/// Use instead:
/// ```python
/// # TODO(ruff): this is now fixed!
/// ```
#[derive(ViolationMetadata)]
pub(crate) struct InvalidTodoTag {
    pub tag: String,
}

impl Violation for InvalidTodoTag {
    #[derive_message_formats]
    fn message(&self) -> String {
        let InvalidTodoTag { tag } = self;
        format!("Invalid TODO tag: `{tag}`")
    }
}

/// ## What it does
/// Checks that a TODO comment includes an author.
///
/// ## Why is this bad?
/// Including an author on a TODO provides future readers with context around
/// the issue. While the TODO author is not always considered responsible for
/// fixing the issue, they are typically the individual with the most context.
///
/// ## Example
/// ```python
/// # TODO: should assign an author here
/// ```
///
/// Use instead
/// ```python
/// # TODO(charlie): now an author is assigned
/// ```
#[derive(ViolationMetadata)]
pub(crate) struct MissingTodoAuthor;

impl Violation for MissingTodoAuthor {
    #[derive_message_formats]
    fn message(&self) -> String {
        "Missing author in TODO; try: `# TODO(<author_name>): ...` or `# TODO @<author_name>: ...`"
            .to_string()
    }
}

/// ## What it does
/// Checks that a TODO comment is associated with a link to a relevant issue
/// or ticket.
///
/// ## Why is this bad?
/// Including an issue link near a TODO makes it easier for resolvers
/// to get context around the issue.
///
/// ## Example
/// ```python
/// # TODO: this link has no issue
/// ```
///
/// Use one of these instead:
/// ```python
/// # TODO(charlie): this comment has an issue link
/// # https://github.com/astral-sh/ruff/issues/3870
///
/// # TODO(charlie): this comment has a 3-digit issue code
/// # 003
///
/// # TODO(charlie): https://github.com/astral-sh/ruff/issues/3870
/// # this comment has an issue link
///
/// # TODO(charlie): #003 this comment has a 3-digit issue code
/// # with leading character `#`
///
/// # TODO(charlie): this comment has an issue code (matches the regex `[A-Z]+\-?\d+`)
/// # SIXCHR-003
/// ```
#[derive(ViolationMetadata)]
pub(crate) struct MissingTodoLink;

impl Violation for MissingTodoLink {
    #[derive_message_formats]
    fn message(&self) -> String {
        "Missing issue link for this TODO".to_string()
    }
}

/// ## What it does
/// Checks that a "TODO" tag is followed by a colon.
///
/// ## Why is this bad?
/// "TODO" tags are typically followed by a parenthesized author name, a colon,
/// a space, and a description of the issue, in that order.
///
/// Deviating from this pattern can lead to inconsistent and non-idiomatic
/// comments.
///
/// ## Example
/// ```python
/// # TODO(charlie) fix this colon
/// ```
///
/// Used instead:
/// ```python
/// # TODO(charlie): colon fixed
/// ```
#[derive(ViolationMetadata)]
pub(crate) struct MissingTodoColon;

impl Violation for MissingTodoColon {
    #[derive_message_formats]
    fn message(&self) -> String {
        "Missing colon in TODO".to_string()
    }
}

/// ## What it does
/// Checks that a "TODO" tag contains a description of the issue following the
/// tag itself.
///
/// ## Why is this bad?
/// TODO comments should include a description of the issue to provide context
/// for future readers.
///
/// ## Example
/// ```python
/// # TODO(charlie)
/// ```
///
/// Use instead:
/// ```python
/// # TODO(charlie): fix some issue
/// ```
#[derive(ViolationMetadata)]
pub(crate) struct MissingTodoDescription;

impl Violation for MissingTodoDescription {
    #[derive_message_formats]
    fn message(&self) -> String {
        "Missing issue description after `TODO`".to_string()
    }
}

/// ## What it does
/// Checks that a "TODO" tag is properly capitalized (i.e., that the tag is
/// uppercase).
///
/// ## Why is this bad?
/// Capitalizing the "TODO" in a TODO comment is a convention that makes it
/// easier for future readers to identify TODOs.
///
/// ## Example
/// ```python
/// # todo(charlie): capitalize this
/// ```
///
/// Use instead:
/// ```python
/// # TODO(charlie): this is capitalized
/// ```
#[derive(ViolationMetadata)]
pub(crate) struct InvalidTodoCapitalization {
    tag: String,
}

impl AlwaysFixableViolation for InvalidTodoCapitalization {
    #[derive_message_formats]
    fn message(&self) -> String {
        let InvalidTodoCapitalization { tag } = self;
        format!("Invalid TODO capitalization: `{tag}` should be `TODO`")
    }

    fn fix_title(&self) -> String {
        let InvalidTodoCapitalization { tag } = self;
        format!("Replace `{tag}` with `TODO`")
    }
}

/// ## What it does
/// Checks that the colon after a "TODO" tag is followed by a space.
///
/// ## Why is this bad?
/// "TODO" tags are typically followed by a parenthesized author name, a colon,
/// a space, and a description of the issue, in that order.
///
/// Deviating from this pattern can lead to inconsistent and non-idiomatic
/// comments.
///
/// ## Example
/// ```python
/// # TODO(charlie):fix this
/// ```
///
/// Use instead:
/// ```python
/// # TODO(charlie): fix this
/// ```
#[derive(ViolationMetadata)]
pub(crate) struct MissingSpaceAfterTodoColon;

impl Violation for MissingSpaceAfterTodoColon {
    #[derive_message_formats]
    fn message(&self) -> String {
        "Missing space after colon in TODO".to_string()
    }
}

static ISSUE_LINK_OWN_LINE_REGEX_SET: LazyLock<RegexSet> = LazyLock::new(|| {
    RegexSet::new([
        r"^#\s*(http|https)://.*", // issue link
        r"^#\s*\d+$",              // issue code - like "003"
        r"^#\s*[A-Z]+\-?\d+$",     // issue code - like "TD003"
    ])
    .unwrap()
});

static ISSUE_LINK_TODO_LINE_REGEX_SET: LazyLock<RegexSet> = LazyLock::new(|| {
    RegexSet::new([
        r"\s*(http|https)://.*", // issue link
        r"\s*#\d+.*",            // issue code - like "#003"
    ])
    .unwrap()
});

pub(crate) fn todos(
    context: &LintContext,
    todo_comments: &[TodoComment],
    locator: &Locator,
    comment_ranges: &CommentRanges,
) {
    for todo_comment in todo_comments {
        let TodoComment {
            directive,
            content,
            range_index,
            ..
        } = todo_comment;
        let range = todo_comment.range;

        // flake8-todos doesn't support HACK directives.
        if matches!(directive.kind, TodoDirectiveKind::Hack) {
            continue;
        }

        directive_errors(context, directive);
        static_errors(context, content, range, directive);

        let mut has_issue_link = false;
        // VSCode recommended links on same line are ok:
        // `# TODO(dylan): #1234`
        if ISSUE_LINK_TODO_LINE_REGEX_SET
            .is_match(locator.slice(TextRange::new(directive.range.end(), range.end())))
        {
            continue;
        }
        let mut curr_range = range;
        for next_range in comment_ranges.iter().skip(range_index + 1).copied() {
            // Ensure that next_comment_range is in the same multiline comment "block" as
            // comment_range.
            if !locator
                .slice(TextRange::new(curr_range.end(), next_range.start()))
                .chars()
                .all(char::is_whitespace)
            {
                break;
            }

            let next_comment = locator.slice(next_range);
            if TodoDirective::from_comment(next_comment, next_range).is_some() {
                break;
            }

            if ISSUE_LINK_OWN_LINE_REGEX_SET.is_match(next_comment) {
                has_issue_link = true;
            }

            // If the next_comment isn't a tag or an issue, it's worthless in the context of this
            // linter. We can increment here instead of waiting for the next iteration of the outer
            // loop.
            curr_range = next_range;
        }

        if !has_issue_link {
            // TD003
            context.report_diagnostic_if_enabled(MissingTodoLink, directive.range);
        }
    }
}

/// Check that the directive itself is valid. This function modifies `diagnostics` in-place.
fn directive_errors(context: &LintContext, directive: &TodoDirective) {
    if directive.content == "TODO" {
        return;
    }

    if directive.content.to_uppercase() == "TODO" {
        // TD006
        if let Some(mut diagnostic) = context.report_diagnostic_if_enabled(
            InvalidTodoCapitalization {
                tag: directive.content.to_string(),
            },
            directive.range,
        ) {
            diagnostic.set_fix(Fix::safe_edit(Edit::range_replacement(
                "TODO".to_string(),
                directive.range,
            )));
        }
    } else {
        // TD001
        context.report_diagnostic_if_enabled(
            InvalidTodoTag {
                tag: directive.content.to_string(),
            },
            directive.range,
        );
    }
}

/// Checks for "static" errors in the comment: missing colon, missing author, etc.
pub(crate) fn static_errors(
    context: &LintContext,
    comment: &str,
    comment_range: TextRange,
    directive: &TodoDirective,
) {
    let post_directive = &comment[usize::from(directive.range.end() - comment_range.start())..];
    let trimmed = post_directive.trim_start();
    let content_offset = post_directive.text_len() - trimmed.text_len();

    let author_end = content_offset
        + if trimmed.starts_with('(') {
            if let Some(end_index) = trimmed.find(')') {
                TextSize::try_from(end_index + 1).unwrap()
            } else {
                trimmed.text_len()
            }
        } else if trimmed.starts_with('@') {
            if let Some(end_index) = trimmed.find(|c: char| c.is_whitespace() || c == ':') {
                TextSize::try_from(end_index).unwrap()
            } else {
                // TD002
                context.report_diagnostic_if_enabled(MissingTodoAuthor, directive.range);

                TextSize::new(0)
            }
        } else {
            // TD002
            context.report_diagnostic_if_enabled(MissingTodoAuthor, directive.range);

            TextSize::new(0)
        };

    let after_author = &post_directive[usize::from(author_end)..];
    if let Some(after_colon) = after_author.strip_prefix(':') {
        if after_colon.is_empty() {
            // TD005
            context.report_diagnostic_if_enabled(MissingTodoDescription, directive.range);
        } else if !after_colon.starts_with(char::is_whitespace) {
            // TD007
            context.report_diagnostic_if_enabled(MissingSpaceAfterTodoColon, directive.range);
        }
    } else {
        // TD004
        context.report_diagnostic_if_enabled(MissingTodoColon, directive.range);

        if after_author.is_empty() {
            // TD005
            context.report_diagnostic_if_enabled(MissingTodoDescription, directive.range);
        }
    }
}
