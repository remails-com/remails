import { Code, Text } from "@mantine/core";
import { useRemails } from "../../hooks/useRemails";

export function SmtpInfo() {
  const {
    state: { config },
  } = useRemails();

  // nicely format list of ports
  const ports =
    config?.smtp_ports.map((p, i, a) => (
      <>
        <Code>{p}</Code>
        {i < a.length - 2 ? ", " : i == a.length - 2 ? ", and " : ""}
      </>
    )) ?? [];

  return (
    <Text>
      These credentials can be used to send emails with your configured domains to the SMTP server hosted at{" "}
      <Code>{config?.smtp_domain_name}</Code> on port{ports.length > 1 ? "s" : ""} {ports}.
    </Text>
  );
}
