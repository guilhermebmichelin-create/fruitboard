import {
  ANALYSIS_LABELS,
  ANALYSIS_NOTES,
  ANALYSIS_REASONS,
  NO_REPORTED_ANALYSIS,
  type ProjectAnalysis,
} from "./projectAnalysis";

export function ProjectAnalysisStatus({
  analysis = NO_REPORTED_ANALYSIS,
  hasFacts,
}: {
  readonly analysis?: ProjectAnalysis | undefined;
  readonly hasFacts: boolean;
}) {
  const retained =
    hasFacts && !["complete", "not_reported"].includes(analysis.state);
  return (
    <div
      className="project-analysis"
      role="group"
      aria-label="Last reported project analysis"
    >
      <h5>Project analysis</h5>
      <p>
        <strong>{ANALYSIS_LABELS[analysis.state]}</strong>
      </p>
      <p>{ANALYSIS_NOTES[analysis.state]}</p>
      {analysis.state === "unsupported" &&
        analysis.savedVersion !== undefined && (
          <p>
            Detected saved FL Studio version:{" "}
            <strong>{analysis.savedVersion}</strong>.
          </p>
        )}
      {analysis.state === "unsupported" && (
        <p>
          This build has not been verified by Fruitboard. This does not
          establish that the project is damaged or its format is incompatible.
          Checking again with this parser does not add support for another
          build.
        </p>
      )}
      {analysis.attempts !== null && (
        <p>Attempts started: {analysis.attempts} of 3.</p>
      )}
      {analysis.reason !== null && <p>{ANALYSIS_REASONS[analysis.reason]}</p>}
      {retained && (
        <p>
          The saved facts below still matched the scanned file when read. They
          are separate from this recorded attempt.
        </p>
      )}
      <p className="project-analysis__note">
        Last reported state. Refresh details to check for a newer result.
        Refresh reads saved results; it does not start another analysis attempt.
      </p>
    </div>
  );
}
