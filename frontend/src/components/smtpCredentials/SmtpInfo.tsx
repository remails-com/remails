import { Group, Table, Text } from "@mantine/core";
import { useRemails } from "../../hooks/useRemails";
import { CopyableCode } from "../CopyableCode.tsx";

export function SmtpInfo() {
  const {
    state: { config },
  } = useRemails();

  if (!config) {
    return null;
  }

  return (
    <>
      <Text>
        These credentials can be used to send emails with your configured domains with the following settings:
      </Text>
      <Group>
        <Table variant="vertical" style={{ width: "auto" }} mt="sm">
          <Table.Tbody>
            <Table.Tr>
              <Table.Th>Server</Table.Th>
              <Table.Td>
                <CopyableCode p="3px">{config?.smtp_domain_name}</CopyableCode>
              </Table.Td>
            </Table.Tr>
            <Table.Tr>
              <Table.Th>Port{config.smtp_ports.length > 1 ? "s" : ""}</Table.Th>
              <Table.Td>{config.smtp_ports.join(", ")}</Table.Td>
            </Table.Tr>
            <Table.Tr>
              <Table.Th>Security</Table.Th>
              <Table.Td>TLS or SSL (no STARTTLS)</Table.Td>
            </Table.Tr>
            <Table.Tr>
              <Table.Th>Supported authentication method</Table.Th>
              <Table.Td>PLAIN</Table.Td>
            </Table.Tr>
          </Table.Tbody>
        </Table>
      </Group>
    </>
  );
}
