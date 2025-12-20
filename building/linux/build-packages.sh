#!/bin/bash

VERSION=$( [ -f ../../../VERSION ] && cat ../../../VERSION || echo "devel" )
RELEASE=1

UID_HOST=$(id -u)
GID_HOST=$(id -g)

top=$(pwd)
# Resolve %top/../..
crate=$(realpath ${top}/../..)

# Debian package creates both udsactor and udsactor-unmanaged packages at once
for debian_version in 12 13; do
    OUTDIR="${top}/../bin/debian${debian_version}"
    rm -f ${OUTDIR}/udsactor*.deb  #  Clean previous debs if any
    # Compile first the binary using rustbuilder.py
    echo "=== Building udsactor binaries using rustbuilder.py ==="
    cd ${top}
    # Always build
    python3 rustbuilder.py Debian${debian_version}
    
    docker_image="rust-builder-udsactor:Debian${debian_version}"
    # Debian based build inside docker
    
    echo "=== Building for Debian ${debian_version} using ${docker_image} ==="
    
    docker run --rm \
        -u ${UID_HOST}:${GID_HOST} \
        -e IN_DOCKER=1 \
        -e DISTRO=Debian${debian_version} \
        -v $crate:/crate \
        -w /crate/building/linux \
        $docker_image \
        dpkg-buildpackage -b -us -uc
    
    # Move to ../bin/debian${debian_version}
    mkdir -p ${OUTDIR}
    mv ${top}/../udsactor*.deb ${OUTDIR}/
done

for DISTRO in Fedora openSUSE; do
    DISTRO_LOWER=$(echo ${DISTRO} | tr '[:upper:]' '[:lower:]')
    RPMROOT=${top}/rpm-${DISTRO_LOWER}
    INSTALLROOT=${top}/rpm-${DISTRO_LOWER}-root

    OUTDIR="${top}/../bin/${DISTRO_LOWER}"
    rm -f ${OUTDIR}/udsactor*.rpm  #  Clean previous rpms if any
    # Clean to enforce recompilation
    make -C ${top} clean \
        IN_DOCKER=0 \
        DISTRO=${DISTRO} \
        DESTDIR=${INSTALLROOT}
    
    for kind in "" "-unmanaged"; do
        # We need to execute manually the Makefile to copy install files
        
        echo "=== Preparing install files for ${DISTRO} ==="
        rm -rf "${INSTALLROOT}"
        mkdir -p ${INSTALLROOT}
        # Re-run to ensure all files are copied
        make -C ${top} install-udsactor${kind} \
            IN_DOCKER=0 \
            DISTRO=${DISTRO} \
            DESTDIR=${INSTALLROOT}
        
        echo "=== Preparing RPM build tree ==="
        rm -rf "${RPMROOT}"
        mkdir -p ${RPMROOT}/{BUILD,RPMS,SOURCES,SPECS,SRPMS}
        cp ${top}/udsactor${kind}.spec ${RPMROOT}/SPECS/udsactor${kind}.spec
        
        docker_image="rust-builder-udsactor:${DISTRO}"
        
        echo "=== Building for ${DISTRO} using ${docker_image} ==="
        
        docker run --rm \
            -u ${UID_HOST}:${GID_HOST} \
            -e DISTRO=${DISTRO} \
            -v $crate:/crate \
            -w /crate/building/linux \
            $docker_image \
            rpmbuild -bb \
                --define "_topdir /crate/building/linux/rpm-${DISTRO_LOWER}" \
                --define "version ${VERSION}" \
                --define "release ${RELEASE}" \
                --define "DESTDIR /crate/building/linux/rpm-${DISTRO_LOWER}-root" \
                /crate/building/linux/rpm-${DISTRO_LOWER}/SPECS/udsactor${kind}.spec
        
        # Move to ../bin/${distro}
        mkdir -p ${OUTDIR}  # Ensure output dir exists
        cp ${top}/rpm-${DISTRO_LOWER}/RPMS/x86_64/udsactor${kind}*.rpm ${OUTDIR}/
        rpm --addsign ${OUTDIR}/udsactor${kind}*.rpm
    done
done

