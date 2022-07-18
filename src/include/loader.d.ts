// Type definitions for Peg Pack grammar files

/**
 * A rule that has been internalized in the grammar through one of the grammar
 * operators. Cannot be constructed directly.
 */
declare class Rule {
  private static readonly BRAND: unique symbol;
  readonly [Rule.BRAND]: unknown;
  private constructor();
}

/**
 * A value that can be treated as a rule when passed to a grammar operator.
 * Function rules are only evaluated once and may be recursive.
 */
type RuleLike = Rule | (() => RuleLike) | string;

/**
 * A continuous range of characters, or a string character, that can be
 * matched as one element.
 */
type Range = [RangeBound, RangeBound] | RangeBound;

/**
 * A bound in a range, either a character code or a string with one character.
 */
type RangeBound = number | string;

/**
 * An interface for creating new rules through parsing operators. Variants of
 * the interface can be constructed that tweak the functionality.
 */
interface GrammarInterface {
  /**
   * Matches if and only if all rules match in sequence. Matches the empty
   * string if no rules are provided.
   */
  readonly seq: (...rules: RuleLike[]) => Rule;

  /**
   * Matches the rule with the furthest error with preference in the order they
   * are listed. If a rule matches without an error, its error distance is
   * considered infinite. Never matches if no rules are provided.
   *
   * The choice operator is associative, but is not commutative.
   */
  readonly choice: (...rules: RuleLike[]) => Rule;

  /**
   * Matches the empty string if none of the provided rules would match,
   * otherwise does not match. Always matches if no rules are provided.
   */
  readonly notAhead: (...rules: RuleLike[]) => Rule;

  /**
   * Matches the provided rule, transforming it into an error if it does match.
   */
  readonly asError: (rule: RuleLike) => Rule;

  /**
   * Matches the provided rule, converting it to an error-free match if it had
   * any errors.
   */
  readonly commit: (rule: RuleLike) => Rule;

  /**
   * Matches the provided rule, wrapping it in a label if it does match.
   */
  readonly label: (label: string, rule: RuleLike) => Rule;

  /**
   * Matches a single character if it appears in any of the provided ranges. If
   * no ranges are given, the rule with never match.
   */
  readonly oneOf: (...ranges: Range[]) => Rule;

  /**
   * Matches a single character if it does not appear in any of the provided
   * ranges. If no ranges are given, the rule will match any character as long
   * as the end of input has not been reached.
   */
  readonly noneOf: (...ranges: Range[]) => Rule;

  /**
   * Matches the empty string if a synchronization rule is ahead, otherwise
   * does not match.
   */
  readonly sync: () => Rule;

  /**
   * Always matches the empty string.
   */
  readonly empty: () => Rule;

  /**
   * Optionally matches one of the provided rules and commits the result.
   *
   * Equivalent to `choice(commit(choice(...rules)), empty)`.
   */
  readonly opt: (...rules: RuleLike[]) => Rule;

  /**
   * Matches the provided rule and then continues to attempt to match the rule
   * until it no longer matches. If the rule matches on the first attempt the
   * entire repetition will match, otherwise it will not. If a separator is
   * provided it will be matched before each additional match of the base rule.
   * Matches of the rule after the first are committed.
   *
   * Equivalent to `more = opt(seq(separator, rule, more)); seq(rule, more)`.
   */
  readonly repOne: (rule: RuleLike, separator?: RuleLike) => Rule;

  /**
   * Matches the provided rule as many times as possible, committing the
   * result. If the rule cannot be matched at all, the empty string is matched.
   * If a separator is provided, it will be matched in between each match of
   * the base rule.
   *
   * Equivalent to `opt(repOne(rule, separator))`.
   */
  readonly rep: (rule: RuleLike, separator?: RuleLike) => Rule;

  /**
   * Constructs a variant of this parser interface whose operators match any
   * number of occurrences of the provided rule between matches of rule
   * operands. This affects the `seq`, `repOne` and `rep` operators but does
   * not affect string literal rules. Rules created with the new interface do
   * not match the whitespace rule before or after the input they match.
   */
  readonly whitespace: (rule: RuleLike) => GrammarInterface;
}

declare global {
  /**
   * The global interface for creating grammar rules. No special behavior, such
   * as whitespace handling, is included. Instead, variant interfaces can be
   * derived from this one that support such behavior.
   */
  const g: GrammarInterface;
}

export {};