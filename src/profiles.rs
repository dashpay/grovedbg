use crate::model::path_display::{Path, PathCtx};

pub(crate) struct Profile<I>(I)
where
    I: Iterator<Item = (Vec<Vec<u8>>, String)> + Clone;

impl<I> Profile<I>
where
    I: Iterator<Item = (Vec<Vec<u8>>, String)> + Clone,
{
    pub fn new(iter: I) -> Self {
        Profile(iter)
    }

    pub fn enable_profile(self, path_ctx: &PathCtx) -> EnabledProfile {
        let mut paths = Vec::new();
        for (path, alias) in self.0.clone() {
            paths.push(path_ctx.add_profiles_alias(path, alias));
        }
        EnabledProfile(paths)
    }
}

pub(crate) struct EnabledProfile<'c>(Vec<Path<'c>>);

impl<'c> EnabledProfile<'c> {
    pub fn disable(self) {
        for path in self.0.into_iter() {
            path.clear_profile_alias();
        }
    }

    pub fn iter_aliases(&self) -> impl Iterator<Item = &Path<'c>> {
        self.0.iter()
    }
}

pub(crate) fn drive_profile() -> Profile<impl Iterator<Item = (Vec<Vec<u8>>, String)> + Clone> {
    Profile::new(
        [
            (vec![vec![64]], "Data contract documents".to_string()),
            (vec![vec![32]], "Identities".to_string()),
            (
                vec![vec![24]],
                "Unique public key hashes to identities".to_string(),
            ),
            (
                vec![vec![8]],
                "Non-unique public key Key hashes to identities".to_string(),
            ),
            (vec![vec![48]], "Pools".to_string()),
            (vec![vec![40]], "Pre funded specialized balances".to_string()),
            (vec![vec![72]], "Spent asset lock transactions".to_string()),
            (vec![vec![104]], "Misc".to_string()),
            (vec![vec![80]], "Withdrawal transactions".to_string()),
            (vec![vec![96]], "Balances".to_string()),
            (vec![vec![16]], "Token balances".to_string()),
            (vec![vec![120]], "Versions".to_string()),
            (vec![vec![112]], "Votes".to_string()),
        ]
        .into_iter(),
    )
}
