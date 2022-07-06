#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::{pallet_prelude::*, BoundedVec};
	use frame_system::pallet_prelude::*;

	use frame_support::dispatch::DispatchResult;
	use frame_support::traits::{
		ChangeMembers, Contains, EnsureOrigin, Get, InitializeMembers, SortedMembers,
	};

	pub type MemberCount = u32;
	pub type Member3Count = u32;

	#[cfg(feature = "std")]
	use frame_support::serde::{Deserialize, Serialize};

	use scale_info::prelude::vec::Vec;

	/// Origin for the ability module.
	#[derive(PartialEq, Eq, Clone, RuntimeDebug, Encode, Decode, TypeInfo, MaxEncodedLen)]
	#[scale_info(skip_type_params(I))]
	#[codec(mel_bound(AccountId: MaxEncodedLen))]
	pub enum RawOrigin<AccountId> {
		Members(MemberCount, MemberCount),
		Member(AccountId),
	}

	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	#[scale_info(skip_type_params(T))]
	#[codec(mel_bound())]
	pub struct SamApp<T: Config> {
		pub name: BoundedVec<u8, T::MaxAppNameLength>,
		pub developer: T::AccountId,
		pub permissions: BoundedVec<u8, T::MaxPermissionsLength>,
	}

	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	#[scale_info(skip_type_params(T))]
	#[codec(mel_bound())]
	pub struct Reviews {
		pub ayes: u32,
		pub nays: u32,
	}

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// Origin allowed to verify apps
		type VerifyOrigin: EnsureOrigin<Self::Origin>;

		type MembershipInitialized: InitializeMembers<Self::AccountId>;

		/// The receiver of the signal for when the membership has changed.
		type MembershipChanged: ChangeMembers<Self::AccountId>;

		#[pallet::constant]
		type MaxAppNameLength: Get<u32>;

		#[pallet::constant]
		type MaxAppCIDLength: Get<u32>;

		#[pallet::constant]
		type MaxPermissionsLength: Get<u32>;

		#[pallet::constant]
		type MaxMembers: Get<u32>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// app submitted for verification
		AppSubmitted(T::AccountId, Vec<u8>, Vec<u8>, Vec<u8>),
		/// regulator has passed a vote
		VerifierVoteCasted(bool),
		/// vote for app has been concluded (App CID, ayes, nays)
		AppReviewConcluded(Vec<u8>, u32, u32),
		/// app has been added to the ability pool
		AddedToAbilityPool(Vec<u8>),
	}

	/// The current membership, stored as an ordered Vec.
	#[pallet::storage]
	#[pallet::getter(fn members)]
	pub type Members<T: Config> =
		StorageValue<_, BoundedVec<T::AccountId, T::MaxMembers>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn verify_status)]
	pub type AppVQueue<T: Config> = StorageMap<
		_,
		Twox64Concat,
		BoundedVec<u8, T::MaxAppCIDLength>,
		BoundedVec<T::AccountId, T::MaxMembers>,
	>;

	#[pallet::storage]
	#[pallet::getter(fn temp_pool)]
	pub(super) type TempPool<T: Config> =
		StorageMap<_, Twox64Concat, BoundedVec<u8, T::MaxAppCIDLength>, SamApp<T>>;

	#[pallet::storage]
	#[pallet::getter(fn ability_pool)]
	pub(super) type AbilityPool<T: Config> =
		StorageMap<_, Twox64Concat, BoundedVec<u8, T::MaxAppCIDLength>, SamApp<T>>;

	#[pallet::storage]
	#[pallet::getter(fn votes)]
	pub(super) type RevQueue<T: Config> =
		StorageMap<_, Twox64Concat, BoundedVec<u8, T::MaxAppCIDLength>, Reviews>;

	#[pallet::storage]
	#[pallet::getter(fn app_count)]
	pub type AppsCount<T: Config> = StorageValue<_, u32>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub members: BoundedVec<T::AccountId, T::MaxMembers>,
		pub phantom: PhantomData<T>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self { members: Default::default(), phantom: Default::default() }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			use sp_std::collections::btree_set::BTreeSet;
			let members_set: BTreeSet<_> = self.members.iter().collect();

			assert_eq!(
				members_set.len(),
				self.members.len(),
				"Members cannot contain duplicate accounts."
			);

			let mut members = self.members.clone();
			members.sort();

			T::MembershipInitialized::initialize_members(&members);
			Members::<T>::put(members);
		}
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// App name length is more than cap
		AppNameLengthOverflow,

		/// App permissions length is more than cap
		PermissionsLengthOverflow,

		/// CID length of app binary is more than cap
		AppCIDOverflow,

		/// Verifier has cast vote previously
		VoteAlreadyCast,

		/// App does not exist
		AppDoesNotExist,

		/// App exists in ability pool already
		AppExistsInPool,

		/// Somehow, the reviews from the app is missing
		ReviewsNotFound,

		/// Not a verifier
		NotAVerifier,

		// Maximum verifier reached
		MaxVerifierReached,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(1000)]
		pub fn upload_app(
			origin: OriginFor<T>,
			app_name: Vec<u8>,
			cid: Vec<u8>,
			perms: Vec<u8>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			// check for overflow
			let app_n: BoundedVec<_, T::MaxAppNameLength> =
				app_name.clone().try_into().map_err(|()| Error::<T>::AppNameLengthOverflow)?;

			let a_cid: BoundedVec<_, T::MaxAppCIDLength> =
				cid.clone().try_into().map_err(|()| Error::<T>::AppCIDOverflow)?;

			let a_perms: BoundedVec<_, T::MaxPermissionsLength> =
				perms.clone().try_into().map_err(|()| Error::<T>::PermissionsLengthOverflow)?;

			// create new ability
			Self::add_ability(&sender, &app_n, &a_cid, &a_perms);

			// register for review
			Self::add_review(&cid);

			Self::deposit_event(Event::AppSubmitted(sender, app_name, cid, perms));

			Ok(())
		}

		#[pallet::weight(1000)]
		pub fn verify_app(origin: OriginFor<T>, app_cid: Vec<u8>, verdict: bool) -> DispatchResult {
			// make sure its only the authorized members
			let verifier = ensure_signed(origin)?;

			// first check cid length
			let cid: BoundedVec<_, T::MaxAppCIDLength> =
				app_cid.clone().try_into().map_err(|()| Error::<T>::AppCIDOverflow)?;

			let members = Members::<T>::get();
			let _location =
				members.binary_search(&verifier).ok().ok_or(Error::<T>::NotAVerifier)?;

			let vstatus = AppVQueue::<T>::get(&cid);

			if vstatus != None {
				// make sure there is no duplicate votes
				let _dv =
					vstatus.binary_search(&verifier).err().ok_or(Error::<T>::VoteAlreadyCast)?;
			}

			// get review queue
			let mut rev = RevQueue::<T>::get(&cid).unwrap_or(Error::<T>::ReviewsNotFound);

			// cast vote
			if verdict == true {
				rev.ayes += 1;
			} else {
				rev.nays += 1;
			}

			// keep value
			let rev_final = rev.clone();

			// commit to memory
			RevQueue::<T>::mutate(&cid, |rv| {
				*rv = Some(rev);
			});

			// generate event to indicate vote
			Self::deposit_event(Event::VerifierVoteCasted(verdict));

			// update the app verification status
			if vstatus == None {
				// if first verifier

				let vs: BoundedVec<_, T::MaxMembers> = vec![verifier];
				AppVQueue::<T>::insert(cid.clone(), vs);
			} else {
				// append new member
				vstatus.try_push(verifier).ok_or(Error::<T>::MaxVerifierReached);
				vstatus.sort();

				// commit to memory
				AppVQueue::<T>::mutate(&cid, |vf| {
					*vf = Some(vstatus);
				});
			}

			// check if it's time for verdict
			if rev_final.ayes + rev_final.nays == T::MaxMembers {
				Self::give_app_verdict(&cid, rev_final.ayes, rev_final.nays);
			}

			Ok(())
		}

		#[pallet::weight(1000)]
		pub fn give_app_verdict(
			origin: OriginFor<T>,
			cid: &BoundedVec<u8, T::MaxAppCIDLength>,
			ayes: u32,
			nays: u32,
		) -> DispatchResult {
			// make sure its only the authorized members
			T::VerifyOrigin::ensure_origin(origin)?;

			if ayes > nays {
				// select app from tempPool
				let app = TempPool::<T>::get(cid).ok_or(<Error<T>>::AppDoesNotExist);

				// insert into ability pool, make sure it hasn't been added before
				ensure!(AbilityPool::<T>::get(cid) == None, Error::<T>::AppExistsInPool);

				// insert into pool
				AbilityPool::<T>::insert(cid.clone(), app.clone());

				// increase total app count
				match AppsCount::<T>::get() {
					Some(count) => AppsCount::<T>::put(count + 1),
					None => AppsCount::<T>::put(1),
				}

				Self::deposit_event(Event::AddedToAbilityPool(cid.to_vec().clone()));
			}

			// remove from temporary pool
			TempPool::<T>::remove(cid);

			// remove review record
			RevQueue::<T>::remove(cid);

			// remove from app verification queue
			AppVQueue::<T>::remove(cid);

			Self::deposit_event(Event::AppReviewConcluded(cid.to_vec().clone(), ayes, nays));

			Ok(())
		}
	}

	/// helper functions
	impl<T: Config> Pallet<T> {
		// upload app for verification in the temporary pool
		pub fn add_ability(
			user_id: &T::AccountId,
			name: &BoundedVec<u8, T::MaxAppNameLength>,
			cid: &BoundedVec<u8, T::MaxAppCIDLength>,
			perm: &BoundedVec<u8, T::MaxPermissionsLength>,
		) {
			let app: SamApp<T> = SamApp {
				name: name.clone(),
				developer: user_id.clone(),
				permissions: perm.clone(),
			};

			// insert into temporary pool
			TempPool::<T>::insert(cid.clone(), app);
		}

		// because new app has been created, register for review
		pub fn add_review(cid: &BoundedVec<u8, T::MaxAppCIDLength>) {
			let rev: Reviews = Reviews { ayes: 0, nays: 0 };

			RevQueue::<T>::insert(cid.clone(), rev);
		}
	}
}
