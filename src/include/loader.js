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
    let ruleName = "<anonymous>";
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

    if (bound < 0 || bound >= 2 ** 32) {
        throw new RangeError("Range bounds must in [0, 2^32)");
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

function seq(g, ...rules) {
    const instructions = rules.map(resolveInstruction);

    if (instructions.length === 0) {
        return createInstruction("empty");
    }

    if (instructions.length === 1) {
        return createInstruction("delegate", { target: instructions[0] });
    }

    if (instructions.length === 2) {
        return createInstruction("seq", { first: instructions[0], second: instructions[1] });
    }

    const [first, ...rest] = rules;
    const restInstruction = seq(g, ...rest);

    return createInstruction("seq", {
        first: resolveInstruction(first),
        second: resolveInstruction(restInstruction),
    });
}

function choice(g, ...rules) {
    const instructions = rules.map(resolveInstruction);

    if (instructions.length === 0) {
        return createInstruction("class", { negated: false, ranges: [] });
    }

    if (instructions.length === 1) {
        return createInstruction("delegate", { target: instructions[0] });
    }

    if (instructions.length === 2) {
        return createInstruction("choice", { first: instructions[0], second: instructions[1] });
    }

    const [first, ...rest] = rules;
    const restInstruction = choice(g, ...rest);

    return createInstruction("choice", {
        first: resolveInstruction(first),
        second: resolveInstruction(restInstruction),
    });
}

function notAhead(g, ...rules) {
    const instruction = resolveInstruction(g.choice(rules));
    return createInstruction("notAhead", { target: instruction });
}

function asError(g, rule) {
    const instruction = resolveInstruction(rule);
    return createInstruction("error", { target: instruction });
}

function label(g, label, rule) {
    if (typeof label !== "string") {
        throw new TypeError("Labels must be a string");
    }

    if (!/[a-z]+(_[a-z]+)*/.test(label)) {
        throw new TypeError(`Labels must be in all lowercase snakecase: ${label}`);
    }

    const instruction = resolveInstruction(rule);
    return createInstruction("label", { target: instruction, label });
}

function oneOf(g, ...ranges) {
    ranges = normalizeRanges(ranges);
    return createInstruction("class", { negated: false, ranges });
}

function noneOf(g, ...ranges) {
    ranges = normalizeRanges(ranges);
    return createInstruction("class", { negated: true, ranges });
}

function empty(_g) {
    return createInstruction("empty");
}

function opt(g, ...instructions) {
    return g.choice(...instructions, g.empty);
}

function repOne(g, rule, separator = g.empty) {
    const more = () => g.opt(g.seq(separator, rule, more));
    anonymousRules.add(more);

    return g.seq(rule, more);
}

function rep(g, rule, separator = g.empty) {
    return g.opt(g.repOne(rule, separator));
}

function whitespace(g, rule) {
    const ws = g.rep(rule);

    const result = { ...g };

    result.seq = (...rules) => {
        const newRules = [];

        for (let i = 0; i < rules.length; i++) {
            if (i !== 0) {
                newRules.push(ws);
            }

            newRules.push(rules[i]);
        }

        return g.seq(...newRules);
    };

    Object.freeze(result);
    return result;
}

function createInterface() {
    const result = {};

    result.seq = seq.bind(null, result);
    result.choice = choice.bind(null, result);
    result.notAhead = notAhead.bind(null, result);
    result.asError = asError.bind(null, result);
    result.label = label.bind(null, result);
    result.oneOf = oneOf.bind(null, result);
    result.noneOf = noneOf.bind(null, result);
    result.empty = empty.bind(null, result);
    result.opt = opt.bind(null, result);
    result.repOne = repOne.bind(null, result);
    result.rep = rep.bind(null, result);
    result.whitespace = whitespace.bind(null, result);

    return result;
}

globalThis.g = createInterface();
Object.freeze(g);

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

