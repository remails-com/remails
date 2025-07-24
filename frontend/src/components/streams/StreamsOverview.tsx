import { Button, Flex, Table } from "@mantine/core";
import { formatDateTime } from "../../util";
import { useStreams } from "../../hooks/useStreams.ts";
import { useRemails } from "../../hooks/useRemails.ts";
import { IconEdit, IconPlus } from "@tabler/icons-react";
import { useDisclosure } from "@mantine/hooks";
import { NewStream } from "./NewStream.tsx";
import { Link } from "../../Link.tsx";
import InfoAlert from "../InfoAlert.tsx";
import StyledTable from "../StyledTable.tsx";

export function StreamsOverview() {
  const [opened, { open, close }] = useDisclosure(false);
  const { navigate } = useRemails();
  const { streams } = useStreams();

  const rows = streams.map((stream) => (
    <Table.Tr key={stream.id}>
      <Table.Td>
        <Link to="projects.project.streams.stream.messages" params={{ stream_id: stream.id }}>
          {stream.name}
        </Link>
      </Table.Td>
      <Table.Td>{formatDateTime(stream.updated_at)}</Table.Td>
      <Table.Td align={"right"}>
        <Button
          variant="subtle"
          onClick={() =>
            navigate("projects.project.streams.stream.settings", {
              stream_id: stream.id,
            })
          }
        >
          <IconEdit />
        </Button>
      </Table.Td>
    </Table.Tr>
  ));

  return (
    <>
      <InfoAlert stateName="streams">
        A Stream functions like an independent SMTP server, with its own credentials. You can create multiple Streams
        within a project to separate traffic for security, performance, or organizational purposes.
      </InfoAlert>
      <NewStream opened={opened} close={close} />

      <StyledTable headers={["Name", "Updated", ""]}>{rows}</StyledTable>
      <Flex justify="center" mt="md">
        <Button onClick={() => open()} leftSection={<IconPlus />}>
          New Stream
        </Button>
      </Flex>
    </>
  );
}
