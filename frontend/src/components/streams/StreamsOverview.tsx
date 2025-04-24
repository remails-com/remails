import {Button, Table} from "@mantine/core";
import {Loader} from "../../Loader";
import {formatDateTime} from "../../util";
import {useStreams} from "../../hooks/useStreams.ts";
import {useRemails} from "../../hooks/useRemails.ts";
import {IconEdit} from "@tabler/icons-react";
import { Project } from "../../types.ts";

export interface StreamsOverviewProps {
  currentProject: Project;
}

export function StreamsOverview({currentProject}: StreamsOverviewProps) {
  const {state: {loading}, navigate} = useRemails();
  const {streams} = useStreams(currentProject);

  if (loading) {
    return <Loader/>;
  }

  const rows = streams.map((stream) => (
    <Table.Tr key={stream.id}>
      <Table.Td>{stream.name}</Table.Td>
      <Table.Td>{formatDateTime(stream.updated_at)}</Table.Td>
      <Table.Td><Button onClick={() => navigate('projects.project.streams.stream', {
        stream_id: stream.id,
      })}><IconEdit/></Button></Table.Td>
    </Table.Tr>
  ));

  return (
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
  );
}