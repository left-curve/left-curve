async function fetchPackageMetadata(packageName, version) {
  try {
    const res = await fetch(`https://registry.npmjs.org/${packageName}`);
    if (!res.ok) return null;
    return res.json();
  } catch (cause) {
    throw new Error(`Failed to fetch metadata for ${packageName}@${version}`, {
      cause,
    });
  }
}

module.exports = {
  hooks: {
    async readPackage(pkg, { log }) {
      const metadata = await fetchPackageMetadata(pkg.name, pkg.version);
      if (
        pkg.name.includes("@left-curve") ||
        pkg.name.includes("webrtc-signaling") ||
        pkg.name.includes("worker-proxy") ||
        pkg.name.includes("leftcurve-monorepo")
      )
        return pkg;
      if (!metadata?.time?.[pkg.version])
        throw new Error(`No publish time found for ${pkg.name}@${pkg.version}`);
      if (metadata?.time?.[pkg.version]) {
        const publishTime = new Date(metadata.time[pkg.version]);
        const daysOld = (Date.now() - publishTime) / (1000 * 60 * 60 * 24);

        if (daysOld < 14) {
          throw new Error(
            `Installation blocked: Package ${pkg.name}@${
              pkg.version
            } is only ${daysOld.toFixed(1)} days old.`,
          );
        }
      }
      log(`Package ${pkg.name}@${pkg.version} passed the age check.`);
      return pkg;
    },
  },
};
