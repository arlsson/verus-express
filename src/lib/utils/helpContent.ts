type TranslateFn = (key: string) => string;

export type HelpQa = {
  id: string;
  question: string;
  answer: string;
};

export type HelpTopic = {
  id: string;
  label: string;
  title: string;
  qas: Array<HelpQa>;
};

export type HelpContent = {
  topics: Array<HelpTopic>;
};

const buildVerusIdGuardTopic = (t: TranslateFn): HelpTopic => ({
  id: 'verus-id-guard',
  label: t('help.topic.verusIdGuard'),
  title: t('help.topic.verusIdGuard'),
  qas: [
    {
      id: 'verus-id-guard-revoke',
      question: t('help.verusIdGuard.revokeQuestion'),
      answer: t('help.verusIdGuard.revokeAnswer')
    },
    {
      id: 'verus-id-guard-funds',
      question: t('help.verusIdGuard.fundsQuestion'),
      answer: t('help.verusIdGuard.fundsAnswer')
    },
    {
      id: 'verus-id-guard-recover',
      question: t('help.verusIdGuard.recoverQuestion'),
      answer: t('help.verusIdGuard.recoverAnswer')
    },
    {
      id: 'verus-id-guard-authority',
      question: t('help.verusIdGuard.authorityQuestion'),
      answer: t('help.verusIdGuard.authorityAnswer')
    }
  ]
});

const buildWalletDifferentTopic = (t: TranslateFn): HelpTopic => ({
  id: 'wallet-different',
  label: t('help.topic.walletDifferent'),
  title: t('help.topic.walletDifferent'),
  qas: [
    {
      id: 'wallet-different-accounts',
      question: t('help.walletDifferent.accountsQuestion'),
      answer: t('help.walletDifferent.accountsAnswer')
    },
    {
      id: 'wallet-different-identity',
      question: t('help.walletDifferent.identityQuestion'),
      answer: t('help.walletDifferent.identityAnswer')
    },
    {
      id: 'wallet-different-payments',
      question: t('help.walletDifferent.paymentsQuestion'),
      answer: t('help.walletDifferent.paymentsAnswer')
    },
    {
      id: 'wallet-different-trust',
      question: t('help.walletDifferent.trustQuestion'),
      answer: t('help.walletDifferent.trustAnswer')
    }
  ]
});

const buildKeepSafeTopic = (t: TranslateFn): HelpTopic => ({
  id: 'keep-safe',
  label: t('help.topic.keepSafe'),
  title: t('help.topic.keepSafe'),
  qas: [
    {
      id: 'keep-safe-items',
      question: t('help.keepSafe.itemsQuestion'),
      answer: t('help.keepSafe.itemsAnswer')
    },
    {
      id: 'keep-safe-phone',
      question: t('help.keepSafe.phoneQuestion'),
      answer: t('help.keepSafe.phoneAnswer')
    },
    {
      id: 'keep-safe-compromised',
      question: t('help.keepSafe.compromisedQuestion'),
      answer: t('help.keepSafe.compromisedAnswer')
    }
  ]
});

const buildLostAccessTopic = (t: TranslateFn): HelpTopic => ({
  id: 'lost-access',
  label: t('help.topic.lostAccess'),
  title: t('help.topic.lostAccess'),
  qas: [
    {
      id: 'lost-access-password',
      question: t('help.lostAccess.passwordQuestion'),
      answer: t('help.lostAccess.passwordAnswer')
    },
    {
      id: 'lost-access-regain',
      question: t('help.lostAccess.regainQuestion'),
      answer: t('help.lostAccess.regainAnswer')
    },
    {
      id: 'lost-access-need',
      question: t('help.lostAccess.needQuestion'),
      answer: t('help.lostAccess.needAnswer')
    }
  ]
});

export function buildNeedHelpContent(
  t: TranslateFn,
  opts: { includeLostAccess: boolean } = { includeLostAccess: false }
): HelpContent {
  const topics: Array<HelpTopic> = [
    buildWalletDifferentTopic(t),
    buildKeepSafeTopic(t),
    buildVerusIdGuardTopic(t)
  ];

  if (opts.includeLostAccess) {
    topics.push(buildLostAccessTopic(t));
  }

  return { topics };
}
