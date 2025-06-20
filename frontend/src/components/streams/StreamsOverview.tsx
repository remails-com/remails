import { Button, Flex, Table } from "@mantine/core";
import { Loader } from "../../Loader";
import { formatDateTime } from "../../util";
import { useStreams } from "../../hooks/useStreams.ts";
import { useRemails } from "../../hooks/useRemails.ts";
import { IconEdit, IconPlus } from "@tabler/icons-react";
import { useDisclosure } from "@mantine/hooks";
import { NewStream } from "./NewStream.tsx";
import { Link } from "../../Link.tsx";

export function StreamsOverview() {
  const [opened, { open, close }] = useDisclosure(false);
  const {
    state: { loading },
    navigate,
  } = useRemails();
  const { streams } = useStreams();

  if (loading || streams === null) {
    return <Loader />;
  }

  const rows = streams.map((stream) => (
    <Table.Tr key={stream.id}>
      <Table.Td>
        <Link
          to="projects.project.streams.stream"
          params={{ stream_id: stream.id }}
          query={{ tab: "messages" }}
        >
          {stream.name}
        </Link>
      </Table.Td>
      <Table.Td>{formatDateTime(stream.updated_at)}</Table.Td>
      <Table.Td align={"right"}>
        <Button
          onClick={() =>
            navigate("projects.project.streams.stream", {
              stream_id: stream.id,
            }, {
              tab: "settings",
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
      <NewStream opened={opened} close={close} />
      <Flex justify="flex-end">
        <Button onClick={() => open()} leftSection={<IconPlus />}>
          New Stream
        </Button>
      </Flex>
      <Table>
        <Table.Thead>
          <Table.Tr>
            <Table.Th>Name</Table.Th>
            <Table.Th>Updated</Table.Th>
            <Table.Th></Table.Th>
          </Table.Tr>
        </Table.Thead>
        <Table.Tbody>{rows}</Table.Tbody>
      </Table>
    </>
  );
}
