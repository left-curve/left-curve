import { createFileRoute } from '@tanstack/react-router'

import { WizardProvider } from '@left-curve/applets-kit'
import { deserializeJson } from '@left-curve/dango/encoding'
import {
  LoginCredentialStep,
  LoginUsernameStep,
  LoginWrapper,
} from '~/components/login'

export const Route = createFileRoute('/(auth)/_auth/login')({
  loader: () => {
    const isFirstVisit = localStorage.getItem('dango.firstVisit')
    return {
      isFirstVisit: !isFirstVisit
        ? true
        : deserializeJson<boolean>(isFirstVisit),
    }
  },
  component: LoginComponent,
})

function LoginComponent() {
  const { isFirstVisit } = Route.useLoaderData()
  return (
    <WizardProvider wrapper={<LoginWrapper isFirstVisit={isFirstVisit} />}>
      <LoginUsernameStep />
      <LoginCredentialStep />
    </WizardProvider>
  )
}
