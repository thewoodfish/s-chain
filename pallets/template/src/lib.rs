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

	// Handles our pallet's currency abstraction
    type AccountOf<T> = <T as frame_system::Config>::AccountId;
    // type BalanceOf<T> =
    //     <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	// #[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	// #[scale_info(skip_type_params(T))]
	// #[codec(mel_bound())]
	// pub struct Samaritan<T: Config> {
	// 	pub sid: AccountOf<T>,
	// 	pub ipfs_cid: [u8; 46],
	// 	pub fund: Option<BalanceOf<T>>
	// }

	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	#[scale_info(skip_type_params(T))]
	#[codec(mel_bound())]
	pub struct SignInDetail<T: Config> {
		pub ip: BoundedVec<u8, T::MaxIPAddress>,
		pub time: u64
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
	}


	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// a Samaritan just signed in
        SignIn(T::AccountId, u64, BoundedVec<u8, T::MaxIPAddress>),
		/// a new Samaritan was just created
		Created(T::AccountId, u64, BoundedVec<u8, T::MaxIPAddress>)
	}

	#[pallet::storage]
    #[pallet::getter(fn sign_ins)]
    pub(super) type SignIns<T: Config> = StorageMap< _, Twox64Concat, T::AccountId, SignInDetail<T>>;

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Samaritan is not existent
        SamaritanNotExist,
		
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// record latest import/creation of Samaritan
		#[pallet::weight(1000)]
		pub fn sign_in(origin: OriginFor<T>, ip_addr: BoundedVec<u8, T::MaxIPAddress>) -> DispatchResult
		{
			// first check for if account id is present
			let sender = ensure_signed(origin)?;
			let user_id = sender.clone();

			let now: u64 = T::TimeProvider::now().as_secs();

			let nsd: SignInDetail<T> = SignInDetail {
				ip: ip_addr.clone(),
				time: now
			};

			// select user
			let detail = <SignIns<T>>::get(&user_id);

			if let Some(_sd) = detail {
				// update timestamp
				<SignIns<T>>::mutate(&user_id, |det| {
					*det = Some(nsd);
				});
			} else {
				// user was just created
				Self::deposit_event(Event::Created(sender, now, ip_addr.clone()));

				// create new signin record and update timestamp 
				<SignIns<T>>::insert(&user_id, nsd);
			}

            Self::deposit_event(Event::SignIn(user_id, now, ip_addr.clone()));

			Ok(())
		}
	
	}

	/// helper functions
	impl<T: Config> Pallet<T> {

	}
}
