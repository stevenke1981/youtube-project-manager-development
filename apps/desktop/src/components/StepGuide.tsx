import type { ReactNode } from "react";

export const WORKFLOW_STEPS = [
  { number: 1, title: "選擇 Library", description: "指定離線素材資料夾" },
  { number: 2, title: "建立影片專案", description: "填寫標題與製作資訊" },
  { number: 3, title: "完成下一個成果", description: "依驗證結果接續製作" }
] as const;

export type StepState = "complete" | "current" | "upcoming";

export function getStepState(stepNumber: number, activeStep: number): StepState {
  if (stepNumber < activeStep) return "complete";
  if (stepNumber === activeStep) return "current";
  return "upcoming";
}

type Props = {
  activeStep: number;
  className?: string;
  footer?: ReactNode;
};

export function StepGuide({ activeStep, className = "", footer }: Props) {
  const nextStep = activeStep <= 1
    ? "Next step: Step 2 · 建立影片專案"
    : activeStep === 2
      ? "Next step: Step 3 · 開啟工作區並完成下一個成果"
      : "Next step: Step 3 · 依 validation 結果接續製作";

  return (
    <div className={`step-guide ${className}`.trim()}>
      <ol aria-label="影片製作流程">
        {WORKFLOW_STEPS.map((step) => {
          const state = getStepState(step.number, activeStep);
          return (
            <li
              className={`step-guide-item ${state}`}
              key={step.number}
              aria-current={state === "current" ? "step" : undefined}
            >
              <span className="step-guide-number">{step.number}</span>
              <span className="step-guide-copy">
                <strong>Step {step.number} · {step.title}</strong>
                <small>{step.description}</small>
              </span>
            </li>
          );
        })}
      </ol>
      <p className="step-guide-next">{nextStep}</p>
      {footer}
    </div>
  );
}
