import { Flex, Table } from "@mantine/core";
import { Loader } from "../../Loader";
import { formatDateTime } from "../../util";
import { useProjects } from "../../hooks/useProjects.ts";
import { IconPlus } from "@tabler/icons-react";
import { useDisclosure } from "@mantine/hooks";
import { NewProject } from "./NewProject.tsx";
import { Link } from "../../Link.tsx";
import EditButton from "../EditButton.tsx";
import StyledTable from "../StyledTable.tsx";
import InfoAlert from "../InfoAlert.tsx";
import OrganizationHeader from "../organizations/OrganizationHeader.tsx";
import { MaintainerButton } from "../RoleButtons.tsx";

export default function ProjectsOverview() {
  const [opened, { open, close }] = useDisclosure(false);
  const { projects } = useProjects();

  if (projects === null) {
    return <Loader />;
  }

  const rows = projects.map((project) => (
    <Table.Tr key={project.id}>
      <Table.Td>
        <Link to="projects.project.streams" params={{ proj_id: project.id }}>
          {project.name}
        </Link>
      </Table.Td>
      <Table.Td>{formatDateTime(project.updated_at)}</Table.Td>
      <Table.Td align={"right"}>
        <EditButton
          route="projects.project.settings"
          params={{
            proj_id: project.id,
          }}
        />
      </Table.Td>
    </Table.Tr>
  ));

  return (
    <>
      <OrganizationHeader />
      <InfoAlert stateName="projects">
        Projects are used to group related work, such as different applications or environments. Each project can have
        its own Streams and Domains to keep things organized.
      </InfoAlert>
      <NewProject opened={opened} close={close} />

      <StyledTable headers={["Name", "Updated", ""]}>{rows}</StyledTable>

      <Flex justify="center" mt="md">
        <MaintainerButton onClick={() => open()} leftSection={<IconPlus />}>
          New Project
        </MaintainerButton>
      </Flex>
    </>
  );
}
