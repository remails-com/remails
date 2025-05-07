import {Modal} from '@mantine/core';
import {NewOrganizationForm} from "./NewOrganizationForm.tsx";


interface NewOrganizationProps {
  opened: boolean;
  close: () => void;
}

export function NewOrganization({opened, close}: NewOrganizationProps) {

  return (
    <>
      <Modal opened={opened} onClose={close} title="Create New Organization">
        <NewOrganizationForm done={close}/>
      </Modal>
    </>
  );
}