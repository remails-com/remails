import {Table} from "@mantine/core";
import {Loader} from "../../Loader";
import {formatDateTime} from "../../util";
import {useStreams} from "../../hooks/useStreams.ts";
import {useRemails} from "../../hooks/useRemails.ts";

export function StreamsOverview() {
  const {state: {loading}} = useRemails();
  const {streams} = useStreams();

  if (loading) {
    return <Loader/>;
  }

  const rows = streams.map((stream) => (
    <Table.Tr key={stream.id}>
      <Table.Td>{stream.name}</Table.Td>
      <Table.Td>{formatDateTime(stream.updated_at)}</Table.Td>
    </Table.Tr>
  ));

  return (
    <Table>
      <Table.Thead>
        <Table.Tr>
          <Table.Th>Name</Table.Th>
          <Table.Th>Updated</Table.Th>
        </Table.Tr>
      </Table.Thead>
      <Table.Tbody>{rows}</Table.Tbody>
    </Table>
  );
}