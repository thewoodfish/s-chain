#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet] 
pub mod pallet {
	use frame_support::dispatch::DispatchResult;
	use frame_support::pallet_prelude::*;
	use frame_system::{pallet_prelude::*}; 

	use frame_support::traits::{tokens::ExistenceRequirement, Currency, UnixTime};
	use frame_support::transactional;

	#[cfg(feature = "std")]
	use frame_support::serde::{Deserialize, Serialize};

	use scale_info::prelude::vec::Vec;

	// Handles our pallet's currency abstraction
    type AccountOf<T> = <T as frame_system::Config>::AccountId;
    // type BalanceOf<T> =
    //     <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	#[scale_info(skip_type_params(T))]
	#[codec(mel_bound())]
	pub struct SignInDetail<T: Config> {
		pub ip: BoundedVec<u8, T::MaxIPAddress>,
		pub time: u64
	}

	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	#[scale_info(skip_type_params(T))]
	#[codec(mel_bound())]
	pub struct Samaritan<T: Config> {
		pub pseudoname: BoundedVec<u8, T::MaxPseudoNameLength>,
		pub cid: BoundedVec<u8, T::MaxCIDLength>
	}

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        // type Currency: Currency<Self::AccountId>;
		type TimeProvider: UnixTime;
 
        #[pallet::constant]
        type MaxIPAddress: Get<u32>;

		#[pallet::constant]
        type MaxPseudoNameLength: Get<u32>;

		#[pallet::constant]
        type MaxCIDLength: Get<u32>;
	}


	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// a Samaritan just signed in
        SignIn(T::AccountId, u64, Vec<u8>),
		/// a new Samaritan was just created
		Created(T::AccountId, u64, Vec<u8>),
		/// the Samaritan state has been modified
		StateModified(T::AccountId, Vec<u8>),
		/// update Samaritan details
		UpdateSamaritan(T::AccountId, Vec<u8>, Vec<u8>)
	}

	#[pallet::storage]
    #[pallet::getter(fn sign_ins)]
    pub(super) type SignIns<T: Config> = StorageMap< _, Twox64Concat, T::AccountId, SignInDetail<T>>;


	#[pallet::storage]
    #[pallet::getter(fn samaritan)]
    pub(super) type SamPool<T: Config> = StorageMap< _, Twox64Concat, T::AccountId, Samaritan<T>>;


	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Samaritan is not existent
        SamaritanNotExist,

		/// IP address length is more than normal
		IPAddressOverflow,

		/// CID address length is more than normal
		CIDAddressOverflow,

		/// Pseudoname length is more than normal
		PseudoNameOverflow,
		
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// record latest import/creation of Samaritan
		#[pallet::weight(1000)]
		pub fn sign_in(origin: OriginFor<T>, ip_addr: Vec<u8>) -> DispatchResult
		{
			// first check for if account id is present
			let sender = ensure_signed(origin)?;
			let user_id = sender.clone();

			// check for overflow
			let ip_address: BoundedVec<_, T::MaxIPAddress> =
				ip_addr.clone().try_into().map_err(|()| Error::<T>::IPAddressOverflow)?;

			// select user
			let detail = SignIns::<T>::get(&user_id);

			let mut now: u64 = 0;

			if let Some(_sd) = detail {
				now = Self::change_si(&user_id, &ip_address);
			} else {
				now = Self::new_si(&user_id, &ip_address);

				// user was just created
				Self::deposit_event(Event::Created(sender, now, ip_addr.clone()));
			}

            Self::deposit_event(Event::SignIn(user_id, now, ip_addr.clone()));

			Ok(())

		}
	
		#[pallet::weight(1000)]
		pub fn change_detail(origin: OriginFor<T>, pseudo: Vec<u8>, cid: Vec<u8>) -> DispatchResult
		{
			let sender = ensure_signed(origin)?;

			// check for overflow
			let ncid: BoundedVec<_, T::MaxCIDLength> =
				cid.clone().try_into().map_err(|()| Error::<T>::CIDAddressOverflow)?;

			let psn: BoundedVec<_, T::MaxPseudoNameLength> =
				cid.clone().try_into().map_err(|()| Error::<T>::PseudoNameOverflow)?;

			// select samaritan
			let sam = SamPool::<T>::get(&sender);

			if let Some(_sam) = sam {
				Self::update_sam(&sender, &psn, &ncid);
			} else {
				Self::new_sam(&sender, &psn, &ncid);
			}

            Self::deposit_event(Event::UpdateSamaritan(sender, pseudo, cid));

			Ok(())
		}
	}

	/// helper functions
	impl<T: Config> Pallet<T> {
		pub fn change_si(user_id: &T::AccountId, ip_address: &BoundedVec<u8, T::MaxIPAddress>) -> u64 {
			let now: u64 = T::TimeProvider::now().as_secs();

			let nsd: SignInDetail<T> = SignInDetail {
				ip: ip_address.clone(),
				time: now
			};

			// update timestamp
			SignIns::<T>::mutate(user_id, |det| {
				*det = Some(nsd);
			});

			now
		}

		pub fn new_si(user_id: &T::AccountId, ip_address: &BoundedVec<u8, T::MaxIPAddress>) -> u64 {
			let now: u64 = T::TimeProvider::now().as_secs();

			let nsd: SignInDetail<T> = SignInDetail {
				ip: ip_address.clone(),
				time: now
			};

			// create new signin record and update timestamp 
			SignIns::<T>::insert(user_id, nsd);

			now
		}

		pub fn update_sam(user_id: &T::AccountId, ps: &BoundedVec<u8, T::MaxPseudoNameLength>, cid: &BoundedVec<u8, T::MaxCIDLength>) {

			let sam: Samaritan<T> = Samaritan {
				pseudoname: ps.clone(),
				cid: cid.clone()
			};

			// update timestamp
			SamPool::<T>::mutate(user_id, |det| {
				*det = Some(sam);
			});

		}

		pub fn new_sam(user_id: &T::AccountId, ps: &BoundedVec<u8, T::MaxPseudoNameLength>, cid: &BoundedVec<u8, T::MaxCIDLength>) {

			let sam: Samaritan<T> = Samaritan {
				pseudoname: ps.clone(),
				cid: cid.clone()
			};

			// create new signin record and update timestamp 
			SamPool::<T>::insert(user_id, sam);

		}
	}
}
