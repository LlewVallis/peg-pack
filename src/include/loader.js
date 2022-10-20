const process = require("process");
const fs = require("fs");

const grammarPath = process.env.PEG_PACK_GRAMMAR;
const irPath = process.env.PEG_PACK_IR;

const ruleNameStack = [];

const instructions = [];
const instructionIds = new Map();
class Instruction {}

class FunctionRuleError extends Error {}

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
        const codePoints = [];

        for (const codePoint of instruction) {
            codePoints.push(codePoint);
        }

        const ranges = codePoints.map(normalizeRange)

        const classes = ranges.map(range => ({
            negated: false,
            ranges: [range],
        }));

        const result = createInstruction("series", { classes });
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

    const hasName = typeof rule.name === "string" && rule.name !== "";

    let result;
    try {
        if (hasName) {
            ruleNameStack.push(rule.name);
        }

        result = resolveInstruction(rule());
    } catch (err) {
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
    const recover = this.recover(...syncs);

    return (...rules) => {
        const transformedRules = rules.map((rule, i) => {
            if (i === 0) {
                return rule;
            } else {
                return recover(rule);
            }
        });

        return this.seq(...transformedRules);
    };
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
    const sync = this.ahead(...syncs, this.eof);

    return (rule) => {
        const result = this.anonymize(() => this.strictChoice(
            rule,
            g.seq(sync, this.error(rule)(this.empty)),
            this.seq(this.error(rule)(this.any), result),
        ));

        const instruction = resolveInstruction(result);
        return createInstruction("delegate", {target: instruction});
    };
}

function ahead(...rules) {
    return this.notAhead(this.notAhead(...rules));
}

function notAhead(...rules) {
    const instruction = resolveInstruction(this.choice(...rules));
    return createInstruction("notAhead", { target: instruction });
}

function error(...expecteds) {
    const expectedInstruction = resolveInstruction(this.choice(...expecteds));

    return rule => {
        const instruction = resolveInstruction(rule);

        return createInstruction("error", {
            target: instruction,
            expected: expectedInstruction,
        });
    };
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
    const more = this.anonymize(() => this.opt(
        this.seq(separator, rule, more)
    ));

    return this.seq(rule, more);
}

function rep(rule, separator = this.empty) {
    if (separator === this.empty) {
        const result = this.anonymize(() => this.opt(this.seq(rule, result)));
        return result;
    } else {
        return this.opt(this.repOne(rule, separator));
    }
}

function untilOne(...syncs) {
    return (rule, separator = this.empty) => {
        const more = this.anonymize(() => this.choice(
            this.ahead(...syncs, this.eof),
            this.seq(
                this.recover(...syncs, rule)(separator),
                this.recover(...syncs)(this.seq(
                    rule,
                    more
                )),
            ),
        ));

        return this.recover(...syncs)(this.seq(rule, more));
    };
}

function until(...syncs) {
    return (rule, separator = this.empty) => {
        const more = this.anonymize(() => this.choice(
            this.ahead(...syncs, this.eof),
            this.seq(
                this.recover(...syncs, rule)(separator),
                this.recover(...syncs)(this.seq(
                    rule,
                    more
                )),
            ),
        ));

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

function anonymize(f) {
    if (!(f instanceof Function)) {
        throw new TypeError("Can only anonymize functions");
    }

    return (...args) => f(...args);
}

function whitespace(...rules) {
    const ws = this.anonymize(() => this.rep(this.choice(...rules)));

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

function tokens(...rules) {
    const base = interfaceBases.get(this);
    const newBase = { ...base };

    newBase.any = () => this.strictChoice(this.choice(...rules), this.any);

    return prepareInterface(newBase);
}

const interfaceBases = new WeakMap();

function prepareInterface(base) {
    const result = {};

    for (const key of Object.keys(base)) {
        result[key] = anonymize(base[key].bind(result));
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
    anonymize,
    whitespace,
    tokens,
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
            message: "The grammar must export an instruction or promise resolving to an instruction",
        };
    }

    fs.writeFileSync(irPath, JSON.stringify(output, null, 4));
})()

