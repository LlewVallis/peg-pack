const ws = g.whitespace(g.choice(" ", "\n", "\r", "\t"));

const value = () => ws.choice(
    object,
    array,
    string,
    number,
    ws.label("null", "null"),
    ws.label("boolean", "true"),
    ws.label("boolean", "false"),
);

const object = () => ws.label("object", ws.seq("{", ws.rep(entry, ","), "}"));

const entry = () => ws.label("entry", ws.seq(
    ws.label("key", string),
    ":",
    ws.label("value", value)
));

const array = () => ws.label("array", ws.seq("[", ws.rep(value, ","), "]"));

const string = () => ws.label("string", g.seq("\"", g.rep(stringCharacter), "\""));

const stringCharacter = () => g.choice(
    g.noneOf("\"", "\\"),
    escapeSequence,
);

const escapeSequence = () => g.seq(
    "\\",
    g.choice(
        g.choice("\"", "\\", "/", "b", "f", "n", "r", "t"),
        g.seq("u", hexDigit, hexDigit, hexDigit, hexDigit),
    )
);

const hexDigit = () => g.oneOf(["0", "9"], ["a", "f"], ["A", "F"]);

const number = () => ws.label("number", g.seq(
    g.opt("-"),
    g.choice("0", g.seq(startDigit, g.rep(digit))),
    g.opt(fractional),
    g.opt(exponent),
));

const fractional = () => g.seq(".", g.repOne(digit));

const exponent = () => g.seq(
    g.choice("e", "E"),
    g.opt(g.choice("+", "-")),
    g.repOne(digit),
);

const startDigit = () => g.oneOf(["1", "9"]);

const digit = () => g.oneOf(["0", "9"]);

module.exports = ws.seq(ws.empty, value, ws.eof);