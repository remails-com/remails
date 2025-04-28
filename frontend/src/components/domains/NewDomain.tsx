import {Button, Group, Loader, Modal, Stack, Stepper, Text, TextInput} from '@mantine/core';
import {useForm} from "@mantine/form";
import {useCurrentOrganization} from "../../hooks/useCurrentOrganization.ts";
import {useRemails} from "../../hooks/useRemails.ts";
import {IconCheck, IconX} from "@tabler/icons-react";
import {notifications} from '@mantine/notifications';
import {useEffect, useState} from "react";
import {Domain} from "../../types.ts";
import {useProjects} from "../../hooks/useProjects.ts";

interface FormValues {
  domain: string,
}

interface NewDomainProps {
  opened: boolean;
  close: () => void;
  projectId: string | null;
}

function validateDomain(domain: string) {
  if (domain.length < 3) {
    return 'Domain must have at least 3 letters'
  }
  if (!domain.includes('.')) {
    return 'Domain must include a top level domain (TLD)'
  }
  return null
}

export function NewDomain({opened, close, projectId}: NewDomainProps) {
  const [activeStep, setActiveStep] = useState(0);
  const currentOrganization = useCurrentOrganization()
  const {currentProject} = useProjects();
  const [newDomain, setNewDomain] = useState<Domain | null>(null);
  const [domainVerified, setDomainVerified] = useState<boolean | null>(null);
  const {navigate, dispatch} = useRemails();

  const form = useForm<FormValues>({
    initialValues: {
      domain: ""
    },
    validate: {
      domain: validateDomain,
    }
  });

  useEffect(() => {
    if (activeStep === 2) {
      setTimeout(() => setDomainVerified(true), 1000);
    } else {
      setDomainVerified(null)
    }
  }, [activeStep]);

  if (!currentOrganization) {
    console.error("Cannot create domain without a selected organization")
    return <></>
  }

  const save = (values: FormValues) => {
    let url = `/api/organizations/${currentOrganization.id}/domains`;
    if (projectId) {
      url = `/api/organizations/${currentOrganization.id}/projects/${projectId}/domains`
    }

    fetch(url, {
      method: 'POST',
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({...values, dkim_key_type: 'rsa_sha256'})
    }).then(res => {
      if (res.status === 201) {
        res.json().then(newDomain => {
          setNewDomain(newDomain)
          dispatch({type: "add_domain", domain: newDomain})
          setActiveStep(1)
        })
      } else if (res.status === 409) {
        form.setFieldError('domain', 'This domain is already configured')
        return
      } else {
        notifications.show({
          title: 'Error',
          message: 'Something went wrong',
          color: 'red',
          autoClose: 20000,
          icon: <IconX size={20}/>,
        })
      }
    })
  }

  const deleteDomain = (domain: Domain | null) => {
    if (!domain) {
      return
    }
    let url = `/api/organizations/${currentOrganization.id}/domains/${domain.id}`;
    if (currentProject) {
      url = `/api/organizations/${currentOrganization.id}/projects/${currentProject.id}/domains/${domain.id}`;
    }
    fetch(url, {
      method: 'DELETE',
    }).then(r => {
      dispatch({type: "remove_domain", domainId: domain.id})
      if (r.status !== 200) {
        notifications.show({
          title: 'Error',
          message: `Something went wrong`,
          color: 'red',
          autoClose: 20000,
          icon: <IconX size={20}/>,
        })
      }
    })
  }

  return (
    <>
      <Modal opened={opened} onClose={close} title="Create New Domain" size="lg">
        <Stepper active={activeStep} onStepClick={setActiveStep}>
          <Stepper.Step label="Create" allowStepSelect={false}>
            <form onSubmit={form.onSubmit(save)}>
              <Stack>
                <TextInput
                  label="Domain Name"
                  key={form.key('domain')}
                  value={form.values.domain}
                  placeholder="example.com"
                  error={form.errors.domain}
                  onChange={(event) => form.setFieldValue('domain', event.currentTarget.value)}
                />
              </Stack>

              <Group justify="space-between" mt="xl">
                <Button onClick={close} variant="outline">Cancel</Button>
                <Button type="submit" loading={form.submitting}>Next</Button>
              </Group>
            </form>
          </Stepper.Step>
          <Stepper.Step label="Configure DNS" allowStepSelect={activeStep >= 1}>
            Please make sure to configure the following DNS records:
            TODO format DNS record correctly
            <Text>
              {newDomain?.dkim_key_type}
            </Text>
            <Text>
              {newDomain?.dkim_public_key}
            </Text>
            <Group justify="space-between">
              <Button variant='outline' onClick={() => {
                setActiveStep(0)
                deleteDomain(newDomain)
                close()
              }}>Cancel</Button>
              <Group>
                <Button onClick={() => close()}>Configure later</Button>
                <Button onClick={() => {
                  setActiveStep(2)
                }}>Verify</Button>
              </Group>
            </Group>
          </Stepper.Step>
          <Stepper.Step label='Verify' allowStepSelect={activeStep >= 1}>
            TODO verify the DNS is configured correctly
            {(domainVerified === null) && <Loader/>}
            {(domainVerified === true) && <IconCheck/>}
            {(domainVerified === false) && <IconX/>}
            <Group justify="flex-end">
              <Button
                disabled={domainVerified === null}
                onClick={() => {
                  setActiveStep(0);
                  close();

                  let route = 'domains.domain';
                  if (currentProject) {
                    route = 'projects.project.domains.domain'
                  }
                  navigate(route, {domain_id: newDomain?.id || ""})
                }}>
                Show {newDomain?.domain}
              </Button>
            </Group>
          </Stepper.Step>
        </Stepper>
      </Modal>
    </>
  )
    ;
}