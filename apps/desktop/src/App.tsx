import { useEffect, useState } from "react";
import { loadRuntimeContract, type RuntimeContract } from "./runtime";
import "./styles.css";

export function App() {
  const [runtime, setRuntime] = useState<RuntimeContract | null>(null);

  useEffect(() => {
    void loadRuntimeContract().then(setRuntime);
  }, []);

  return (
    <main className="app-shell">
      <section className="status-card" aria-labelledby="product-title">
        <p className="eyebrow">Fully offline clinical decision support</p>
        <h1 id="product-title">CHEW Companion</h1>
        <p>
          Guided support for CHEWs and JCHEWs based on Nigeria&apos;s 2024
          National Standing Orders.
        </p>
        {runtime ? (
          <p>Supported cadres: {runtime.supportedCadres.join(" + ")}</p>
        ) : null}
        <p className="safety-note">
          This tool supports and does not replace clinical judgment.
        </p>
      </section>
    </main>
  );
}
