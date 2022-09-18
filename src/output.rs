//! Rust code generation API

pub struct Codegen {
    buffer: String,
    indent: usize,
    new_line: bool,
}

impl Codegen {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            indent: 0,
            new_line: true,
        }
    }

    pub fn finish(mut self) -> String {
        let trimmed = self.buffer.trim_end();
        let new_len = trimmed.len();
        self.buffer.truncate(new_len);

        self.buffer.push_str("\n");
        self.buffer
    }

    pub fn line(&mut self, content: &str) {
        self.write(content);
        self.newline();
    }

    pub fn function(&mut self, signature: &str) -> Statements {
        self.line("#[allow(unused)]");
        self.write(signature);
        self.space();
        self.open_brace();

        Statements::new(self, |codegen| {
            codegen.close_brace();
            codegen.newline();
        })
    }

    pub fn enumeration(&mut self, name: &str, public: bool) -> Enum {
        if public {
            self.write("pub ");
        }

        self.write("enum ");
        self.write(name);
        self.space();
        self.open_brace();

        Enum { codegen: self }
    }

    pub fn trait_impl(&mut self, name: &str, target: &str) -> Trait {
        self.write("impl ");
        self.write(name);
        self.write(" for ");
        self.write(target);
        self.space();
        self.open_brace();

        Trait {
            codegen: self,
            first: false,
        }
    }

    fn open_brace(&mut self) {
        self.indent();
        self.write("{");
        self.newline();
    }

    fn close_brace(&mut self) {
        self.dedent();
        self.write("}");
        self.newline();
    }

    pub fn indent(&mut self) {
        self.indent += 1;
    }

    pub fn dedent(&mut self) {
        self.indent -= 1;
    }

    pub fn newline(&mut self) {
        self.trim_spaces();
        self.write("\n");
        self.new_line = true;
    }

    fn space(&mut self) {
        self.write(" ");
    }

    fn trim_spaces(&mut self) {
        let trimmed = self.buffer.trim_end_matches(" ");
        let new_length = trimmed.len();
        self.buffer.truncate(new_length);
    }

    fn write(&mut self, content: &str) {
        if self.new_line {
            for _ in 0..self.indent {
                self.buffer.push_str("    ");
            }

            self.new_line = false;
        }

        self.buffer.push_str(content);
    }
}

pub struct Statements<'a> {
    codegen: &'a mut Codegen,
    finish: Option<fn(&mut Codegen)>,
}

impl<'a> Statements<'a> {
    fn new(codegen: &'a mut Codegen, finish: fn(&mut Codegen)) -> Self {
        Self {
            codegen,
            finish: Some(finish),
        }
    }

    pub fn line(&mut self, content: &str) {
        self.codegen.line(content)
    }

    pub fn newline(&mut self) {
        self.codegen.newline();
    }

    pub fn match_statement(&mut self, control: &str) -> Match {
        Match::new(self.codegen, control)
    }

    pub fn if_statement(&mut self, control: &str) -> Statements {
        self.codegen.write("if ");
        self.codegen.write(control);
        self.codegen.space();
        self.codegen.open_brace();

        Statements::new(self.codegen, Codegen::close_brace)
    }
}

impl<'a> Drop for Statements<'a> {
    fn drop(&mut self) {
        let finish = self.finish.take().unwrap();
        finish(self.codegen);
    }
}

pub struct Match<'a> {
    codegen: &'a mut Codegen,
}

impl<'a> Match<'a> {
    fn new(codegen: &'a mut Codegen, control: &str) -> Self {
        codegen.write("match ");
        codegen.write(control);
        codegen.space();
        codegen.open_brace();

        Self { codegen }
    }

    pub fn case_line(&mut self, pattern: &str, line: &str) {
        self.codegen.write(pattern);
        self.codegen.write(" => ");
        self.codegen.write(line);
        self.codegen.line(",");
    }
}

impl<'a> Drop for Match<'a> {
    fn drop(&mut self) {
        self.codegen.close_brace();
    }
}

pub struct Enum<'a> {
    codegen: &'a mut Codegen,
}

impl<'a> Enum<'a> {
    pub fn variant(&mut self, name: &str) {
        self.codegen.write(name);
        self.codegen.line(",");
    }
}

impl<'a> Drop for Enum<'a> {
    fn drop(&mut self) {
        self.codegen.close_brace();
        self.codegen.newline();
    }
}

pub struct Trait<'a> {
    codegen: &'a mut Codegen,
    first: bool,
}

impl<'a> Trait<'a> {
    pub fn function(&mut self, signature: &str) -> Statements {
        if self.first {
            self.codegen.newline();
        } else {
            self.first = true;
        }

        self.codegen.line("#[allow(unused)]");
        self.codegen.write(signature);
        self.codegen.space();
        self.codegen.open_brace();

        Statements::new(self.codegen, |codegen| {
            codegen.close_brace();
        })
    }
}

impl<'a> Drop for Trait<'a> {
    fn drop(&mut self) {
        self.codegen.close_brace();
        self.codegen.newline();
    }
}
