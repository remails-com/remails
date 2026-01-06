import { Flex, Pagination, Stack, Table } from "@mantine/core";
import { Loader } from "../../Loader";
import { formatDateTime } from "../../util";
import { useProjects } from "../../hooks/useProjects.ts";
import { IconPlus } from "@tabler/icons-react";
import { useDisclosure, useScrollIntoView } from "@mantine/hooks";
import { NewProject } from "./NewProject.tsx";
import { Link } from "../../Link.tsx";
import EditButton from "../EditButton.tsx";
import StyledTable from "../StyledTable.tsx";
import InfoAlert from "../InfoAlert.tsx";
import OrganizationHeader from "../organizations/OrganizationHeader.tsx";
import { MaintainerButton } from "../RoleButtons.tsx";
import { useState } from "react";

const PER_PAGE = 20;

export default function ProjectsOverview() {
  const [opened, { open, close }] = useDisclosure(false);
  const { projects } = useProjects();
  const [activePage, setPage] = useState(1);

  const { scrollIntoView, targetRef } = useScrollIntoView<HTMLTableSectionElement>({
    duration: 500,
    offset: 100,
  });

  if (projects === null) {
    return <Loader />;
  }

  const rows = projects.slice((activePage - 1) * PER_PAGE, activePage * PER_PAGE).map((project) => (
    <Table.Tr key={project.id}>
      <Table.Td>
        <Link to="projects.project.emails" params={{ proj_id: project.id }}>
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
        Projects are used to group related work, such as different applications or environments.
      </InfoAlert>
      <NewProject opened={opened} close={close} />

      <StyledTable ref={targetRef} headers={["Name", "Updated", ""]}>
        {rows}
      </StyledTable>

      <Flex justify="center" mt="md">
        <Stack>
          {projects.length > PER_PAGE && (
            <Pagination
              value={activePage}
              onChange={(p) => {
                setPage(p);
                scrollIntoView({ alignment: "start" });
              }}
              total={Math.ceil(projects.length / PER_PAGE)}
            />
          )}
          <MaintainerButton onClick={() => open()} leftSection={<IconPlus />}>
            New Project
          </MaintainerButton>
        </Stack>
      </Flex>
    </>
  );
}
