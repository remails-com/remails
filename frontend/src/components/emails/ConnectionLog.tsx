import { Code, Modal, Table } from "@mantine/core";
import { Log } from "../../types.ts";

interface ConnectionLogProps {
  log: Log;
  opened: boolean;
  close: () => void;
}

function formatTime(t: string | Date): string {
  const time = new Date(t);
  const iso = time.toISOString();
  return `${iso.slice(0, 10)} ${iso.slice(11, 19)}`;
}

export function ConnectionLog({ log, close, opened }: ConnectionLogProps) {
  return (
    <>
      <Modal opened={opened} onClose={close} title="Connection log" size={1500} onClick={(e) => e.stopPropagation()}>
        <Code block>
          <Table withRowBorders={false} variant="vertical" verticalSpacing="0">
            <Table.Tbody>
              {log.lines.map((line, i) => (
                <Table.Tr key={i}>
                  <Table.Td style={{ whiteSpace: "nowrap", width: "1%" }}>{formatTime(line.time)}</Table.Td>
                  <Table.Td style={{ whiteSpace: "nowrap", width: "1%" }}>{line.level}</Table.Td>
                  <Table.Td style={{ whiteSpace: "pre" }}>{line.msg}</Table.Td>
                </Table.Tr>
              ))}
            </Table.Tbody>
          </Table>
        </Code>
      </Modal>
    </>
  );
}
