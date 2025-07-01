import { Button, Flex, Table } from "@mantine/core";
import { Loader } from "../../Loader";
import { formatDateTime } from "../../util";
import { useProjects } from "../../hooks/useProjects.ts";
import { useRemails } from "../../hooks/useRemails.ts";
import { IconEdit, IconPlus } from "@tabler/icons-react";
import { useDisclosure } from "@mantine/hooks";
import { NewProject } from "./NewProject.tsx";
import { Link } from "../../Link.tsx";

export default function ProjectsOverview() {
  const [opened, { open, close }] = useDisclosure(false);
  const { navigate } = useRemails();
  const { projects } = useProjects();

  if (projects === null) {
    return <Loader />;
  }

  const rows = projects.map((project) => (
    <Table.Tr key={project.id}>
      <Table.Td>
        <Link to="projects.project" params={{ proj_id: project.id, tab: "streams" }}>
          {project.name}
        </Link>
      </Table.Td>
      <Table.Td>{formatDateTime(project.updated_at)}</Table.Td>
      <Table.Td align={"right"}>
        <Button
          variant="subtle"
          onClick={() =>
            navigate("projects.project", {
              proj_id: project.id,
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
      <NewProject opened={opened} close={close} />
      <Table highlightOnHover>
        <Table.Thead>
          <Table.Tr>
            <Table.Th>Name</Table.Th>
            <Table.Th>Updated</Table.Th>
            <Table.Th></Table.Th>
          </Table.Tr>
        </Table.Thead>
        <Table.Tbody>{rows}</Table.Tbody>
      </Table>
      <Flex justify="center" mt="md">
        <Button onClick={() => open()} leftSection={<IconPlus />}>
          New Project
        </Button>
      </Flex>
    </>
  );
}
