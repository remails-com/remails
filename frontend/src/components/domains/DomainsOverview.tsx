import {useRemails} from "../../hooks/useRemails.ts";
import {useDomains} from "../../hooks/useDomains.ts";
import {Loader} from "../../Loader.tsx";
import {Button, Flex, Table} from "@mantine/core";
import {formatDateTime} from "../../util.ts";
import {IconEdit, IconPencilPlus} from "@tabler/icons-react";
import {useDisclosure} from "@mantine/hooks";
import {NewDomain} from "./NewDomain.tsx";
import {useProjects} from "../../hooks/useProjects.ts";

export default function DomainsOverview() {
  const [opened, {open, close}] = useDisclosure(false);
  const {state: {loading}, navigate} = useRemails();
  const {currentProject} = useProjects();
  const {domains} = useDomains();

  if (loading || domains === null) {
    return <Loader/>;
  }

  const rows = domains.map((domain) => (
    <Table.Tr key={domain.id}>
      <Table.Td>{domain.domain}</Table.Td>
      <Table.Td>{formatDateTime(domain.updated_at)}</Table.Td>
      <Table.Td align={'right'}>
        <Button
          onClick={() => {
            let route = 'domains.domain';
            if (currentProject) {
              route = 'projects.project.domains.domain';
            }
            navigate(route, {
              domain_id: domain.id,
            })
          }}>
          <IconEdit/></Button>
      </Table.Td>
    </Table.Tr>
  ));

  return (
    <>
      <NewDomain opened={opened} close={close} projectId={currentProject?.id || null}/>
      <Flex justify="flex-end">
        <Button onClick={() => open()} leftSection={<IconPencilPlus/>}> New Domain</Button>
      </Flex>
      <Table>
        <Table.Thead>
          <Table.Tr>
            <Table.Th>Domain</Table.Th>
            <Table.Th>Updated</Table.Th>
            <Table.Th></Table.Th>
          </Table.Tr>
        </Table.Thead>
        <Table.Tbody>{rows}</Table.Tbody>
      </Table>
    </>
  );
}