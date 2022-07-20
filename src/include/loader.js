const process = require("process");
const fs = require("fs");

const grammarPath = process.env.PEG_PACK_GRAMMAR;
const irPath = process.env.PEG_PACK_IR;

const ruleNameStack = [];

const instructions = [];
const instructionIds = new Map();
class Instruction {}

let errored = false;

class FunctionRuleError extends Error {}

const anonymousRules = new WeakSet();

function createInstruction(name, object) {
    const id = instructions.length;
    instructions.push(buildInstruction(name, object));
    const result = new Instruction();
    instructionIds.set(result, id);
    return result;
}

function buildInstruction(name, object) {
    let ruleName = undefined;
    if (ruleNameStack.length > 0) {
        ruleName = ruleNameStack[ruleNameStack.length - 1];
    }

    return { ...object, name, ruleName };
}

function resolveInstruction(instruction) {
    if (instructionIds.has(instruction)) {
        return instructionIds.get(instruction);
    } else if (instruction instanceof Function) {
        return resolveFunctionRule(instruction);
    } else if (typeof instruction === "string") {
        let result = g.empty;

        for (const codePoint of instruction) {
            result = g.seq(result, g.oneOf(codePoint));
        }

        const id = resolveInstruction(result);
        instructionIds.set(instruction, id);
        return id;
    } else {
        throw new TypeError(`Invalid instruction: ${instruction}`);
    }
}

function hasInstructionType(instruction) {
    return instruction instanceof Instruction
        || instruction instanceof Function
        || typeof instruction === "string";
}

function resolveFunctionRule(rule) {
    const id = instructions.length;
    instructionIds.set(rule, id);
    instructions.push(null);

    const hasName = typeof rule.name === "string"
        && rule.name !== ""
        && !anonymousRules.has(rule);

    let result;
    try {
        if (hasName) {
            ruleNameStack.push(rule.name);
        }

        result = resolveInstruction(rule());
    } catch (err) {
        errored = true;
        rethrowFunctionRuleError(err);
    } finally {
        instructions[id] = buildInstruction("delegate", { target: result });

        if (hasName) {
            ruleNameStack.pop();
        }
    }

    return id;
}

function rethrowFunctionRuleError(err) {
    const stack = [...ruleNameStack]
        .reverse()
        .map(name => `'${name}'`)
        .join(", ");

    let newErr;
    if (err instanceof FunctionRuleError) {
        newErr = err;
    } else if (ruleNameStack.length > 1) {
        newErr = new FunctionRuleError(`Exception in rule: ${stack}`);
        newErr.cause = err;
    } else {
        newErr = new FunctionRuleError("Exception in unnamed rule");
        newErr.cause = err;
    }

    throw newErr;
}

function normalizeBound(bound) {
    if (typeof bound === "string") {
        if ([...bound].length !== 1) {
            throw new RangeError("Range bound strings must have one character");
        }

        bound = bound.codePointAt(0);
    }

    if (typeof bound !== "number") {
        throw new TypeError("Range bounds must be numbers or characters");
    }

    if (!Number.isInteger(bound)) {
        throw new RangeError("Range bounds must be integers");
    }

    if (bound < 0 || bound > 255) {
        throw new RangeError("Range bounds must be between 0 and 255");
    }

    return bound;
}

function normalizeRange(range) {
    if (typeof range === "number" || typeof range === "string") {
        range = [range, range];
    }

    if (!(range instanceof Array)) {
        throw new TypeError("Ranges must be arrays");
    }

    if (range.length !== 2) {
        throw new RangeError("Range arrays must be of length 2");
    }

    const start = normalizeBound(range[0]);
    const end = normalizeBound(range[1]);

    if (start > end) {
        throw new RangeError("A range's first bound cannot exceed its second");
    }

    return [start, end];
}

function normalizeRanges(ranges) {
    if (!(ranges instanceof Array)) {
        throw new TypeError("Ranges must be an array");
    }

    return ranges.map(normalizeRange);
}

function seq(...rules) {
    const instructions = rules.map(resolveInstruction);

    let result = g.empty;

    for (const instruction of instructions) {
        const resultInstruction = resolveInstruction(result);
        result = createInstruction("seq", { first: resultInstruction, second: instruction });
    }

    return result;
}

function then(...syncs) {
    const f = (...rules) => {
        const transformedRules = rules.map((rule, i) => {
            if (i === 0) {
                return rule;
            } else {
                return this.recover(...syncs)(rule);
            }
        });

        return this.seq(...transformedRules);
    };

    anonymousRules.add(f);

    return f;
}

function choice(...rules) {
    const instructions = rules.map(resolveInstruction);

    let result = g.never;

    for (const instruction of instructions) {
        const resultInstruction = resolveInstruction(result);
        result = createInstruction("choice", { first: resultInstruction, second: instruction });
    }

    return result;
}

function strictChoice(...rules) {
    let result = g.never;

    for (const rule of rules) {
        const strictInstruction = resolveInstruction(g.seq(g.notAhead(result), rule));
        const resultInstruction = resolveInstruction(result);
        result = createInstruction("choice", { first: resultInstruction, second: strictInstruction });
    }

    return result;
}

function recover(...syncs) {
    const f = (rule) => {
        const sync = this.ahead(...syncs, this.eof);

        const result = () => this.strictChoice(
            rule,
            g.seq(sync, this.error(this.empty)),
            this.seq(this.error(this.any), result),
        );

        anonymousRules.add(result);

        const instruction = resolveInstruction(result);
        return createInstruction("delegate", { target: instruction });
    };

    anonymousRules.add(f);

    return f;
}

function ahead(...rules) {
    return this.notAhead(this.notAhead(...rules));
}

function notAhead(...rules) {
    const instruction = resolveInstruction(this.choice(...rules));
    return createInstruction("notAhead", { target: instruction });
}

function error(rule) {
    const instruction = resolveInstruction(rule);
    return createInstruction("error", { target: instruction });
}

function label(label, rule) {
    if (typeof label !== "string") {
        throw new TypeError("Labels must be a string");
    }

    if (!/[a-z]+(_[a-z]+)*/.test(label)) {
        throw new TypeError(`Labels must be in all lowercase snakecase: ${label}`);
    }

    const instruction = resolveInstruction(rule);
    return createInstruction("label", { target: instruction, label });
}

function oneOf(...ranges) {
    ranges = normalizeRanges(ranges);
    return createInstruction("series", {
        classes: [{ negated: false, ranges }]
    });
}

function noneOf(...ranges) {
    ranges = normalizeRanges(ranges);
    return createInstruction("series", {
        classes: [{ negated: true, ranges }]
    });
}

function empty() {
    return createInstruction("series", { classes: [] });
}

function never() {
    return createInstruction("series", {
        classes: [{ negated: false, ranges: [] }]
    });
}

function opt(...rules) {
    return this.strictChoice(
        this.choice(...rules),
        this.empty
    );
}

function repOne(rule, separator = this.empty) {
    const more = () => this.opt(this.seq(separator, rule, more));
    anonymousRules.add(more);

    return this.seq(rule, more);
}

function rep(rule, separator = this.empty) {
    return this.opt(this.repOne(rule, separator));
}

function untilOne(...syncs) {
    return (rule, separator = this.empty) => {
        const more = () => this.choice(
            this.ahead(...syncs, this.eof),
            this.seq(
                this.recover(...syncs, rule)(separator),
                this.recover(...syncs)(this.seq(
                    rule,
                    more
                )),
            ),
        );

        anonymousRules.add(more);

        return this.recover(...syncs)(this.seq(rule, more));
    };
}

function until(...syncs) {
    return (rule, separator = this.empty) => {
        const more = () => this.choice(
            this.ahead(...syncs, this.eof),
            this.seq(
                this.recover(...syncs, rule)(separator),
                this.recover(...syncs)(this.seq(
                    rule,
                    more
                )),
            ),
        );

        anonymousRules.add(more);

        return this.recover(...syncs)(this.choice(
            this.ahead(...syncs, this.eof),
            this.seq(rule, more)
        ));
    };
}

function any() {
    return this.noneOf();
}

function eof() {
    return this.notAhead(this.any);
}

function whitespace(rule) {
    const ws = this.rep(rule);

    const base = interfaceBases.get(this);
    const newBase = { ...base };

    const oldSeq = this.seq;
    newBase.seq = (...rules) => {
        const newRules = [];

        for (let i = 0; i < rules.length; i++) {
            if (i !== 0) {
                newRules.push(ws);
            }

            newRules.push(rules[i]);
        }

        return oldSeq(...newRules);
    };

    return prepareInterface(newBase);
}

const interfaceBases = new WeakMap();

function prepareInterface(base) {
    const result = {};

    for (const key of Object.keys(base)) {
        result[key] = base[key].bind(result);
        anonymousRules.add(result[key]);
    }

    interfaceBases.set(result, base);
    Object.freeze(result);

    return result;
}

globalThis.g = prepareInterface({
    seq,
    then,
    choice,
    strictChoice,
    recover,
    ahead,
    notAhead,
    error,
    label,
    oneOf,
    noneOf,
    empty,
    never,
    opt,
    repOne,
    rep,
    untilOne,
    until,
    any,
    eof,
    whitespace,
});

process.on("uncaughtException", err => {
    console.error("Uncaught exception:", err);
    process.exit(1);
});

const grammar = require(grammarPath);

(async () => {
    const result = await grammar;

    let output;
    if (hasInstructionType(result)) {
        const start = resolveInstruction(result);

        output = {
            version: 0,
            status: "success",
            instructions,
            start,
        };
    } else {
        output = {
            version: 0,
            status: "error",
            error: "The grammar must export an instruction or promise resolving to an instruction",
        };
    }

    fs.writeFileSync(irPath, JSON.stringify(output, null, 4));
})()

