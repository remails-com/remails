import { DomainVerificationResult } from "../../types.ts";
import { Badge, Tooltip } from "@mantine/core";

export default function VerificationBadge({ status }: { status: DomainVerificationResult | null }) {
  const errors = [];
  const warnings = [];

  if (!status) {
    return (
      <Badge color="gray" style={{ cursor: "pointer" }}>
        Unverified
      </Badge>
    );
  }

  for (const key of ["dkim", "spf", "dmarc", "a"] as const) {
    const value = status[key];
    if (value.status == "Error") {
      errors.push(key.toUpperCase());
    }
    if (value.status == "Warning") {
      warnings.push(key.toUpperCase());
    }
  }

  if (errors.length > 0) {
    let label = `${errors.join(", ")} record errors`;
    if (warnings.length > 0) {
      label += `, and ${warnings.join(", ")} record warnings`;
    }
    return (
      <Tooltip label={label}>
        <Badge style={{ cursor: "pointer" }}>Error</Badge>
      </Tooltip>
    );
  }

  if (warnings.length > 0) {
    return (
      <Tooltip label={`${warnings.join(", ")} record warnings`}>
        <Badge color="orange" style={{ cursor: "pointer" }}>
          Warning
        </Badge>
      </Tooltip>
    );
  }

  return (
    <Badge color="green" style={{ cursor: "pointer" }}>
      Verified
    </Badge>
  );
}
