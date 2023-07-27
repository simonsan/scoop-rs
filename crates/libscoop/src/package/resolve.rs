use crate::{
    error::Fallible,
    event,
    internal::dag::DepGraph,
    package::{query, Package, QueryOption},
    Error, Session,
};

/// Resolve dependencies of the given packages.
pub(crate) fn resolve_dependencies(session: &Session, packages: &mut Vec<Package>) -> Fallible<()> {
    let mut graph = DepGraph::<String>::new();
    let mut to_resolve = packages.clone();

    loop {
        if to_resolve.is_empty() {
            break;
        }

        let mut tmp = vec![];
        tmp.append(&mut to_resolve);

        for pkg in tmp.into_iter() {
            let mut resolved = vec![];
            let deps = pkg.dependencies();

            if deps.is_empty() {
                graph.register_node(pkg.name().to_owned());
            } else {
                let queries = deps.iter().map(|d| d.as_str());

                for query in queries {
                    let mut matched =
                        query::query_synced(session, query, &[QueryOption::Explicit])?;

                    match matched.len() {
                        0 => return Err(Error::PackageNotFound(query.to_owned())),
                        1 => resolved.push(matched.pop().unwrap()),
                        _ => {
                            select_candidate(session, &mut matched)?;
                            resolved.push(matched.pop().unwrap());
                        }
                    }
                }

                let dep_nodes = resolved
                    .iter()
                    .map(|p: &Package| p.name().to_owned())
                    .collect::<Vec<_>>();
                graph.register_deps(pkg.name().to_owned(), dep_nodes);
            }
            // Cyclic dependency check
            graph.check()?;

            resolved.dedup();
            to_resolve.append(&mut resolved);
        }

        packages.extend(to_resolve.clone());
    }

    packages.dedup();
    // dependencies need to be installed before dependents
    packages.reverse();

    Ok(())
}

/// Select one from multiple package candidates, interactively if possible.
pub(crate) fn select_candidate(session: &Session, candidates: &mut Vec<Package>) -> Fallible<()> {
    // Try to filter out strictly installed ones if possible. Only the strictly
    // installed one because we may support replacement, that is to say, migration
    // of installed packages by choosing a candidate with a same name but
    // different bucket.
    candidates.retain(|p| !p.is_strictly_installed());

    // Luckily, there is no more than one package left
    if candidates.len() <= 1 {
        return Ok(());
    }

    let name = candidates[0].name().to_owned();

    // Sort candidates by package ident, in other words, by alphabetical order
    // of bucket name.
    candidates.sort_by_key(|p| p.ident());

    // Only we can ask user/frontend to select one from multiple candidates
    // when the outbound tx is available for us to do an interactive q&a.
    if let Some(tx) = session.emitter() {
        let question = candidates.iter().map(|p| p.ident()).collect::<Vec<_>>();

        if tx.send(event::Event::SelectPackage(question)).is_ok() {
            // The unwrap is safe here because we have obtained the outbound tx,
            // so the inbound rx must be available.
            let rx = session.receiver().unwrap();

            while let Ok(answer) = rx.recv() {
                if let event::Event::SelectPackageAnswer(idx) = answer {
                    // bounds check
                    if idx < candidates.len() {
                        *candidates = vec![candidates[idx].clone()];

                        return Ok(());
                    }

                    return Err(Error::InvalidAnswer);
                }
            }
        }
    }

    // TODO: handle this case smartly using pre-defined bucket priority
    Err(Error::PackageMultipleCandidates(name))
}

/// Resolve unneeded dependencies of the given packages.
///
/// This function is used to resolve the unneeded dependencies of the given
/// packages. The unneeded dependencies are the dependencies that are not
/// depended by other installed packages.
///
/// The purpose is to support cascading removal of installed packages.
pub(crate) fn resolve_cascade(session: &Session, packages: &mut Vec<Package>) -> Fallible<()> {
    let mut to_resolve = packages.clone();
    let installed = query::query_installed(session, "*", &[QueryOption::Explicit])?;

    loop {
        if to_resolve.is_empty() {
            break;
        }

        let tmp = to_resolve.clone();
        to_resolve = vec![];

        for pkg in tmp.into_iter() {
            // unneeded: the packages that are not depended by other installed
            // packages.
            let mut unneeded = vec![];

            let dep_names = pkg
                .dependencies()
                .into_iter()
                .map(super::extract_name)
                .collect::<Vec<_>>();

            for dep_name in dep_names {
                let mut result = installed
                    .iter()
                    .filter(|p| p.name() == dep_name)
                    .collect::<Vec<_>>();

                // The package dependency system of Scoop is not mandatory,
                // the dependency relationship is loose. For the original
                // Scoop implementation, it is allowed that a dependency may
                // be removed separately without checking its dependents.
                // This can cause the empty result of the query.
                if result.is_empty() {
                    continue;
                }

                // We queried the installed packages, it is impossible to
                // have more than one result here for an explicit package
                // name.
                assert_eq!(result.len(), 1);

                let dep_pkg = result.pop().unwrap();
                // The dependency package may be depended by other installed
                // packages.
                let mut dependents = vec![];
                installed.iter().for_each(|p| {
                    let be_dependent = p
                        .dependencies()
                        .iter()
                        .map(super::extract_name)
                        .any(|d| d == dep_pkg.name());
                    if be_dependent {
                        dependents.push(p.clone());
                    }
                });

                // `pkg` is already the package to be removed, not counted.
                dependents.retain(|p| p.name() != pkg.name());

                let needed = dependents
                    .iter()
                    .any(|p| !packages.contains(p) && !unneeded.contains(p));

                if !needed {
                    unneeded.push(dep_pkg.to_owned());
                }
            }

            unneeded.dedup();
            to_resolve.append(&mut unneeded);
        }

        packages.extend(to_resolve.clone());
    }

    packages.dedup();

    Ok(())
}
