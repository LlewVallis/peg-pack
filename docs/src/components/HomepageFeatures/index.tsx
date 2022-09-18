import React from "react";
import clsx from "clsx";
import styles from "./styles.module.css";

type FeatureItem = {
  title: string;
  description: JSX.Element;
};

const FeatureList: FeatureItem[] = [
  {
    title: "Powerful grammars",
    description: (
      <>
        All grammars are lowered to a minimalistic intermediate representation
        before being analyzed by the generator. You can build your own
        primitives on top of Peg Pack's base semantics depending on the needs of
        your language.
      </>
    ),
  },
  {
    title: "Input format agnostic",
    description: (
      <>
        Peg Pack parsers operate directly on a byte array giving you fine
        control over encoding sensitive behavior and allowing you to parse
        binary formats. You can implicitly tokenize directly in the parser or
        parse a token stream built externally.
      </>
    ),
  },
  {
    title: "Javascript DSL",
    description: (
      <>
        Write grammars directly in Javascript where you can freely define your
        own combinators to implement otherwise tedious patterns. This also saves
        you learning a new language and installing a plugin for editor support.
      </>
    ),
  },
  {
    title: "Error handling",
    description: (
      <>
        The DSL comes with a set of builtin set of error recovery tools that
        compose nicely. If you don't like the default error handling behavior,
        you can use lower level constructs to tweak the exact semantics.
      </>
    ),
  },
];

function Feature({ title, description }: FeatureItem) {
  return (
    <div className={clsx("col")}>
      <div className="text--left padding-horiz--sm">
        <h3>{title}</h3>
        <p>{description}</p>
      </div>
    </div>
  );
}

export default function HomepageFeatures(): JSX.Element {
  return (
    <section className={styles.features}>
      <div className="container">
        <div className="row">
          {FeatureList.map((props, idx) => (
            <Feature key={idx} {...props} />
          ))}
        </div>
      </div>
    </section>
  );
}
