#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::{pallet_prelude::*, BoundedVec};
	use frame_system::pallet_prelude::*;

	use frame_support::dispatch::DispatchResult;
	use frame_support::traits::{Contains, EnsureOrigin, Get, InitializeMembers};

	#[cfg(feature = "std")]
	use frame_support::serde::{Deserialize, Serialize};

	use scale_info::prelude::vec::Vec;
	use sp_std::vec;

	#[derive(Default, Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	#[scale_info(skip_type_params(T))]
	#[codec(mel_bound())]
	pub struct SamApp<T: Config> {
		pub name: BoundedVec<u8, T::MaxAppNameLength>,
		pub cid: BoundedVec<u8, T::MaxAppCIDLength>,
		pub developer: T::AccountId,
		pub permissions: BoundedVec<u8, T::MaxPermissionsLength>,
		pub downloads: u64
	}


	#[derive(Default, Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	#[scale_info(skip_type_params(T))]
	#[codec(mel_bound())]
	pub struct Permissions<T: Config> {
		pub account: T::AccountId,
		pub allow: bool
	}

	#[derive(Default, Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
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
		// type VerifyOrigin: EnsureOrigin<Self::Origin>;

		type MembershipInitialized: InitializeMembers<Self::AccountId>;

		/// The receiver of the signal for when the membership has changed.
		// type MembershipChanged: ChangeMembers<Self::AccountId>;

		#[pallet::constant]
		type MaxAppNameLength: Get<u32>;

		#[pallet::constant]
		type MaxAppCIDLength: Get<u32>;

		#[pallet::constant]
		type MaxPermissionsLength: Get<u32>;

		#[pallet::constant]
		type MaxMembers: Get<u32>;

		#[pallet::constant]
		type MaxNetworkApps: Get<u32>;

		#[pallet::constant]
		type MaxAppDownloads: Get<u32>;
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
		/// app successfully downloaded
		AppDownloadSuccessful(Vec<u8>, T::AccountId, u64),
		/// app download record modified
		DownloadRecordModified(T::AccountId, Vec<u8>)
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

	/// list containing all the apps passed by the network
	#[pallet::storage]
	#[pallet::getter(fn app_list)]
	pub type AppsList<T: Config> = StorageValue<_, BoundedVec<SamApp<T>, T::MaxNetworkApps>>;

	/// apps mapped to the users that downloads them
	#[pallet::storage]
	#[pallet::getter(fn downloads)]
	pub type Downloads<T: Config> = 
		StorageMap<_, Twox64Concat, BoundedVec<u8, T::MaxAppCIDLength>, BoundedVec<Permissions<T>, T::MaxAppDownloads>>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub members: Vec<T::AccountId>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self { members: Vec::new() }
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
			// T::MembershipInitialized::initialize_members(&members);

			let genesis_members: BoundedVec<T::AccountId, T::MaxMembers> =
				self.members.clone().try_into().expect("Too many genesis members");
			Members::<T>::set(genesis_members);
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

		/// Maximum verifier reached
		MaxVerifierReached,

		/// Too many members
		MembersOverflow,

		/// Maximum number of apps allowed reached
		MaxNetworkAppsBoundaryReached,

		/// Maximum number of app dowload reached
		MaxAppsDownloadReached,

		/// A record was not found
		RecordNotFound
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
			Self::add_review(&a_cid);

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
			// let _location =
			// 	members.binary_search(&verifier).ok().ok_or(Error::<T>::NotAVerifier)?;

			if !members.to_vec().contains(&verifier) {
				// throw error
				ensure!(true == false, Error::<T>::NotAVerifier);
			}

			let vstatus = AppVQueue::<T>::get(&cid);

			// unwrapped version
			let mut vs_unwrapped = vstatus.clone().unwrap_or_default();
 
			if vstatus != None {
				// make sure there is no duplicate votes
				let _dv = vs_unwrapped
					.binary_search(&verifier)
					.err()
					.ok_or(Error::<T>::VoteAlreadyCast)?;
			}

			// get review queue
			let mut rev = RevQueue::<T>::get(&cid).unwrap_or_default();

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
				let first_v = vec![verifier];
				let vs: BoundedVec<_, T::MaxMembers> =
					first_v.clone().try_into().map_err(|()| Error::<T>::MembersOverflow)?;

				AppVQueue::<T>::insert(cid.clone(), vs);
			} else {
				// append new member
				vs_unwrapped.try_push(verifier);
				vs_unwrapped.sort();

				// commit to memory
				AppVQueue::<T>::mutate(&cid, |vf| {
					*vf = Some(vs_unwrapped);
				});
			}

			// check if it's time for verdict
			if rev_final.ayes + rev_final.nays == T::MaxMembers::get() {
				Self::give_app_verdict(&cid, rev_final.ayes, rev_final.nays)?;
			}

			Ok(())
		}


		// modify app permissions for a user
		#[pallet::weight(1000)]
		pub fn change_permissions(origin: OriginFor<T>, app_cid: Vec<u8>, allow: bool) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			// record the addition of the app to the users state
			let cid: BoundedVec<_, T::MaxAppCIDLength> =
					app_cid.clone().try_into().map_err(|()| Error::<T>::AppCIDOverflow)?;

			Self::modify_download_record(&sender, &cid, allow);

			Ok(())
		}

		#[pallet::weight(1000)]
		pub fn download_app(origin: OriginFor<T>, app_cid: Vec<u8>) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			// record the addition of the app to the users state
			let cid: BoundedVec<_, T::MaxAppCIDLength> =
					app_cid.clone().try_into().map_err(|()| Error::<T>::AppCIDOverflow)?;

			// first check if the record exists
			if let None = Downloads::<T>::get(&cid) {
				// create new record
				let empty: Vec<Permissions<T>> = vec![];
				let bounded_empty: BoundedVec<_, T::MaxAppDownloads> = 
					empty.try_into().map_err(|()| Error::<T>::MaxAppsDownloadReached)?;

				Downloads::<T>::insert(cid.clone(), bounded_empty);
			}

			Self::modify_download_record(&sender, &cid, false)?;

			let mut app = AbilityPool::<T>::get(&cid).unwrap();
			let download_count = app.downloads.clone();
			app.downloads += 1;
			AbilityPool::<T>::try_mutate(&cid, |app_x| {
				if true {
					*app_x = Some(app);
					return Ok(0);
				}

				Err(0)
			});

			Self::deposit_event(Event::AppDownloadSuccessful(app_cid, sender, download_count));

			Ok(())
		}
	}

	/// helper functions;
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
				cid: cid.clone(),
				developer: user_id.clone(),
				permissions: perm.clone(),
				downloads: 0
			};

			// insert into temporary pool
			TempPool::<T>::insert(cid.clone(), app);
		}

		// because new app has been created, register for review
		pub fn add_review(cid: &BoundedVec<u8, T::MaxAppCIDLength>) {
			let rev: Reviews = Reviews { ayes: 0, nays: 0 };

			RevQueue::<T>::insert(cid.clone(), rev);
		}

		pub fn give_app_verdict(
			cid: &BoundedVec<u8, T::MaxAppCIDLength>,
			ayes: u32,
			nays: u32,
		) -> Result<(), Error<T>> {
			// make sure its only the authorized members
			// T::VerifyOrigin::ensure_origin(origin)?;

			if ayes > nays {
				// select app from tempPool
				let app = TempPool::<T>::get(cid).ok_or(<Error<T>>::AppDoesNotExist);
				let app_new = app.unwrap();
				let app_new1 = TempPool::<T>::get(cid).ok_or(<Error<T>>::AppDoesNotExist).unwrap();

				// insert into ability pool, make sure it hasn't been added before
				if let Some(_appv) = AbilityPool::<T>::get(cid) {
					// throw error
					ensure!(true == false, Error::<T>::AppExistsInPool)
				}

				// insert into pool
				AbilityPool::<T>::insert(cid.clone(), app_new);

				// increase total app count
				match AppsCount::<T>::get() {
					Some(count) => AppsCount::<T>::put(count + 1),
					None => AppsCount::<T>::put(1),
				}

				// add app to apps list
				let mut list = AppsList::<T>::get().unwrap_or_default();
				list.try_push(app_new1).map_err(|()| Error::<T>::MaxNetworkAppsBoundaryReached)?;

				AppsList::<T>::put(list);

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

		pub fn modify_download_record(
			sender: &T::AccountId,
			cid: &BoundedVec<u8, T::MaxAppCIDLength>,
			allow: bool
		) -> Result<(), Error<T>> {
			
			let dp: Permissions<T> = Permissions {
				account: sender.clone(),
				allow
			};

			// now just select the record
			let app_d = Downloads::<T>::get(&cid);
			let mut downloads = app_d.unwrap_or_default();

			downloads.try_push(dp).map_err(|_| Error::<T>::MaxAppsDownloadReached)?;
			Downloads::<T>::try_mutate(&cid, |dl| {
				if true {
					*dl = Some(downloads);
					return Ok(0);
				}

				Err(0)
			});

			Self::deposit_event(Event::DownloadRecordModified(sender.clone(), cid.to_vec().clone()));

			Ok(())
		}
	}
}
