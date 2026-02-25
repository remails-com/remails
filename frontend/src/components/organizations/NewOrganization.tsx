import { Modal, Title } from "@mantine/core";
import { NewOrganizationForm } from "./NewOrganizationForm.tsx";
import { Organization } from "../../types.ts";

interface NewOrganizationProps {
  opened: boolean;
  close: () => void;
  done?: (newOrg: Organization) => void;
}

export function NewOrganization({ opened, close, done }: NewOrganizationProps) {
  return (
    <>
      <Modal
        opened={opened}
        onClose={close}
        title={
          <Title order={3} component="span">Create new organization</Title>
        }
        size="lg"
        padding="xl"
      >
        <NewOrganizationForm
          done={(newOrg: Organization) => {
            if (done) {
              done(newOrg);
            }
            close();
          }}
          close={close}
        />
      </Modal>
    </>
  );
}
