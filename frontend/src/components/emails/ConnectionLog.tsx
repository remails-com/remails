import { Code, Modal, Table, Title } from "@mantine/core";
import { DeliveryDetails } from "../../types.ts";
import { Recipient } from "./Recipients.tsx";

interface ConnectionLogProps {
  details: DeliveryDetails;
  recipient: string;
  opened: boolean;
  close: () => void;
}

function formatTime(t: string | Date): string {
  const time = new Date(t);
  const iso = time.toISOString();
  return `${iso.slice(0, 10)} ${iso.slice(11, 19)}`;
}

export function ConnectionLog({ details, recipient, close, opened }: ConnectionLogProps) {
  return (
    <Modal opened={opened} onClose={close} title={
      <Title order={3}>
        Connection log
        <Recipient details={details} recipient={recipient} props={{ ml: "xl" }} />
      </ Title>
    } size={1500} onClick={(e) => e.stopPropagation()}>
      <Code block>
        <Table withRowBorders={false} variant="vertical" verticalSpacing="0" highlightOnHover>
          <Table.Tbody>
            {details.log.lines.map((line, i) => (
              <Table.Tr key={i}>
                <Table.Td style={{ whiteSpace: "nowrap", width: "1%", verticalAlign: "top" }}>{formatTime(line.time)}</Table.Td>
                <Table.Td style={{ whiteSpace: "nowrap", width: "1%", verticalAlign: "top" }}>{line.level}</Table.Td>
                <Table.Td style={{ whiteSpace: "normal" }}>{line.msg}</Table.Td>
              </Table.Tr>
            ))}
          </Table.Tbody>
        </Table>
      </Code>
    </Modal>
  );
}
