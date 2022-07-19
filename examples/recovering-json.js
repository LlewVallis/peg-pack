const ws = g.whitespace(g.choice(" ", "\n", "\r", "\t"));

const recover = () => ws.choice("}", "]", ",", value);

const value = () => ws.choice(
    object,
    array,
    string,
    number,
    ws.label("null", "null"),
    ws.label("boolean", "true"),
    ws.label("boolean", "false"),
);

const object = () => ws.label("object", ws.then()(
    "{",
    ws.until("}")(entry, ","),
    "}"
));

const entry = () => ws.label("entry", ws.then(recover)(
    ws.label("key", string),
    ":",
    ws.label("value", value),
));

const array = () => ws.label("array", ws.then()(
    "[",
    ws.until("]")(value, ","),
    "]",
));

const string = () => ws.label("string", g.then()(
    "\"",
    g.until("\"")(stringCharacter),
    "\"",
));

const stringCharacter = () => g.choice(
    g.noneOf([0, 31], [127, 255], "\"", "\\"),
    escapeSequence,
);

const escapeSequence = () => g.seq(
    "\\",
    g.choice(
        g.choice("\"", "\\", "/", "b", "f", "n", "r", "t"),
        g.seq("u", hexDigit, hexDigit, hexDigit, hexDigit),
    ),
);

const hexDigit = () => g.choice(
    g.oneOf(["0", "9"], ["a", "f"], ["A", "F"]),
    g.error(g.choice(g.noneOf("\\", "\""), g.empty)),
);

const number = () => ws.label("number", g.seq(
    g.opt("-"),
    g.choice(
        "0",
        g.seq(startDigit, g.rep(digit))
    ),
    g.opt(fractional),
    g.opt(exponent),
));

const fractional = () => g.then(recover)(".", g.repOne(digit));

const exponent = () => g.then(recover)(
    g.choice("e", "E"),
    g.opt("+", "-"),
    g.repOne(digit),
);

const startDigit = () => g.oneOf(["1", "9"]);

const digit = () => g.oneOf(["0", "9"]);

module.exports = ws.then()(ws.empty, value, ws.eof);
