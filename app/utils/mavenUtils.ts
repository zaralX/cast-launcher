export function getMavenUrl(name: string, url: string) {
    const path = getMavenLibraryPath(name);
    return url.endsWith('/') ? url + path : url + '/' + path;
}

export function getMavenLibraryPath(name: string): string {
    const [groupId, artifactId, version] = name.split(':');
    return `${groupId!.replace(/\./g, '/')}/${artifactId}/${version}/${artifactId}-${version}.jar`;
}