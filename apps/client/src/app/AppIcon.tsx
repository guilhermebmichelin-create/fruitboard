import type { LucideIcon } from "lucide-react";

interface AppIconProps {
  readonly icon: LucideIcon;
  readonly size?: "small" | "medium" | "large";
}

export function AppIcon({ icon: Icon, size = "medium" }: AppIconProps) {
  return (
    <Icon
      aria-hidden="true"
      className={`app-icon app-icon--${size}`}
      focusable="false"
      strokeWidth={1.8}
    />
  );
}
